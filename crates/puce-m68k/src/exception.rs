//! Exception processing: the group 1 and 2 frame shared by privilege violation, TRAP, CHK, ...
//! (`Cpu::exception`), the interrupt sequence (`Cpu::take_interrupt`) and the reset sequence
//! (`Cpu::reset`).
//!
//! Frame (6 bytes): PC low at SSP-2, SR at SSP-6, PC high at SSP-4, then SSP -= 6, the vector is
//! read (long) at `4 * vector` and two prefetches refill the queue at the handler.

use crate::cpu::Cpu;
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::regs::{S, T};

/// Hook: saves SR and the PC of the instruction at IR, enters supervisor mode (T cleared).
pub const HK_PRIV: u8 = 0xF0;
/// Hook: address and data of the first push (PC low word).
pub const HK_PUSH_PC_LO: u8 = 0xF1;
/// Hook: address and data of the second push (old SR).
pub const HK_PUSH_SR: u8 = 0xF2;
/// Hook: address and data of the third push (PC high word); updates the supervisor stack pointer.
pub const HK_PUSH_PC_HI: u8 = 0xF3;
/// Hook: address of the exception vector.
pub const HK_VECTOR: u8 = 0xF4;
/// Hook: PC = vector read.
pub const HK_JUMP: u8 = 0xF5;
/// Hook: selects vector 8 (privilege violation).
pub const HK_SET_PRIV: u8 = 0xF6;
/// Hook: start of an interrupt: like `HK_PRIV` and the interrupt mask takes the level.
pub const HK_INT_START: u8 = 0xF7;
/// Hook: address of the interrupt acknowledge cycle (A1-A3 = level, other lines high).
pub const HK_IACK: u8 = 0xF8;
/// Hook: the vector number read during the acknowledge cycle.
pub const HK_VEC_ACK: u8 = 0xF9;
/// Hook: start of the reset: SR = 0x2700, first vector at address 0.
pub const HK_RST_BEGIN: u8 = 0xFA;
/// Hook: SSP = vector 0, next vector at address 4.
pub const HK_RST_SSP: u8 = 0xFB;

/// Hook: selects the vector of the instruction at IRD (TRAP #n: 32 + n, TRAPV: 7, CHK: 6).
pub const HK_VEC_INSN: u8 = 0xFC;
/// Hook: the pushed PC is the address of the word after the opcode (`PC - 2`).
pub const HK_PC_NEXT: u8 = 0xFD;
/// Hook: saves SR and enters supervisor mode before the first prefetch (TRAPV).
pub const HK_SUP_EARLY: u8 = 0xFE;
/// Hook: the pushed PC is `PC - 4`, taken after the prefetch.
pub const HK_PC_SET: u8 = 0xFF;

/// Hook: next word of the address error frame (`Latch::ae`), see [`ae_push`].
pub const HK_AE_PUSH: u8 = 0xE0;

/// Vector number of the privilege violation.
pub const VEC_PRIVILEGE: u8 = 8;
/// First autovector (level 1 is vector 25).
pub const VEC_AUTOVECTOR: u8 = 24;

static PRIVILEGE: [Tok; 15] = [
    Tok::H(HK_SET_PRIV),
    Tok::N(4),
    Tok::H(HK_PRIV),
    Tok::H(HK_PUSH_PC_LO),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_SR),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_PC_HI),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_VECTOR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

static EXCEPTION: [Tok; 14] = [
    Tok::N(4),
    Tok::H(HK_PRIV),
    Tok::H(HK_PUSH_PC_LO),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_SR),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_PC_HI),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_VECTOR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

// Interrupt (44 clocks, Yacht: `n nn ns ni n- n nS ns nV nv np n np`): the idle cycles around the
// acknowledge are not observable by an oracle (no SST case), their split is a decision (D-042).
static INTERRUPT: [Tok; 17] = [
    Tok::N(6),
    Tok::H(HK_INT_START),
    Tok::H(HK_PUSH_PC_LO),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_IACK),
    Tok::Ia,
    Tok::H(HK_VEC_ACK),
    Tok::N(4),
    Tok::H(HK_PUSH_SR),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_PC_HI),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_VECTOR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
];

static RESET: [Tok; 9] = [
    Tok::H(HK_RST_BEGIN),
    Tok::N(14),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_RST_SSP),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

/// Builds an exception plan: the given head (vector, idle cycles, `HK_PRIV`, ...) then the
/// common frame push, vector fetch and refill.
macro_rules! frame_plan {
    ($($h:expr),*) => {
        [
            $($h,)*
            Tok::H(HK_PUSH_PC_LO),
            Tok::Ww(AddrSrc::Ea),
            Tok::H(HK_PUSH_SR),
            Tok::Ww(AddrSrc::Ea),
            Tok::H(HK_PUSH_PC_HI),
            Tok::Ww(AddrSrc::Ea),
            Tok::H(HK_VECTOR),
            Tok::Rl(AddrSrc::Ea),
            Tok::H(HK_JUMP),
            Tok::Pf,
            Tok::N(2),
            Tok::Pf,
        ]
    };
}

static TRAP: [Tok; 16] = frame_plan![
    Tok::H(HK_VEC_INSN),
    Tok::N(4),
    Tok::H(HK_PRIV),
    Tok::H(HK_PC_NEXT)
];
static TRAPV: [Tok; 16] = frame_plan![
    Tok::H(HK_SUP_EARLY),
    Tok::Pf,
    Tok::H(HK_VEC_INSN),
    Tok::H(HK_PC_SET)
];
static CHK_POS: [Tok; 16] = frame_plan![
    Tok::H(HK_VEC_INSN),
    Tok::N(10),
    Tok::H(HK_PRIV),
    Tok::H(HK_PC_NEXT)
];
static CHK_NEG: [Tok; 16] = frame_plan![
    Tok::H(HK_VEC_INSN),
    Tok::N(8),
    Tok::H(HK_PRIV),
    Tok::H(HK_PC_NEXT)
];

/// TRAP #n (34 clocks): `n nn ns ns ns nV nv np n np`.
pub fn trap_plan() -> Plan {
    &TRAP
}

/// TRAPV with V set (34 clocks): supervisor mode is entered before `np`, which takes the place\n/// of the first idle cycle (the oracle fetches it with FC = supervisor program).
pub fn trapv_plan() -> Plan {
    &TRAPV
}

/// CHK trap (`negative`: Dn < 0, 40 clocks; else Dn > bound, 38 clocks, idle cycles included).
pub fn chk_trap_plan(fast: bool) -> Plan {
    if fast {
        &CHK_NEG
    } else {
        &CHK_POS
    }
}

/// Plan of the privilege violation exception (34 clocks): `nn ns ns ns nV nv np n np`.
pub fn privilege_violation() -> Plan {
    &PRIVILEGE
}

/// Generic exception plan (34 clocks), vector number in `Latch::vec`: `nn ns ns ns nV nv np n np`.
pub fn exception_plan() -> Plan {
    &EXCEPTION
}

// Address error (50 clocks after the aborted cycle): `n(8) ns ns ns ns ns ns ns nV nv np n np`;
// the frame has 14 bytes (status, access address, IR, SR, PC).
static ADDRESS_ERROR: [Tok; 21] = [
    Tok::N(8),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_AE_PUSH),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_VECTOR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

/// Plan of the address error exception, after the aborted bus cycle (vector 3 in `Latch::vec`).
pub fn address_error_plan() -> Plan {
    &ADDRESS_ERROR
}

/// Pushes the next word of the address error frame: PC low, SR, PC high, IR, access address low,
/// status word, access address high (`Latch::ae` = PC, SR, IR, address, status).
pub fn ae_push(cpu: &mut Cpu) {
    let ssp = cpu.regs.ssp;
    let [pc, sr, ir, addr, status, _] = cpu.lat.ae;
    let (off, val) = match cpu.lat.ae_step {
        0 => (2, pc & 0xFFFF),
        1 => (6, sr),
        2 => (4, pc >> 16),
        3 => (8, ir),
        4 => (10, addr & 0xFFFF),
        5 => (14, status),
        _ => (12, addr >> 16),
    };
    cpu.lat.ea = ssp.wrapping_sub(off);
    cpu.lat.val = val;
    if cpu.lat.ae_step == 6 {
        cpu.regs.ssp = ssp.wrapping_sub(14);
    }
    cpu.lat.ae_step += 1;
}

/// PC pushed by an address error: the 68000 stacks a value that depends on the instruction, its
/// addressing mode and how far the prefetch had gone (table derived from the oracle, D-052).
/// `ipc` is the address of the instruction, `addr` the faulting address, `write` the direction.
pub fn ae_pc(ird: u16, write: bool, ipc: u32, addr: u32) -> u32 {
    let base = ipc.wrapping_add(2);
    let mode = ((ird >> 3) & 7) as u8;
    let reg = (ird & 7) as u8;
    // Extension words of an addressing mode.
    let words = |m: u8, r: u8| -> u32 {
        match (m, r) {
            (5 | 6, _) | (7, 0 | 2 | 3) => 1,
            (7, 1) => 2,
            _ => 0,
        }
    };
    let hi = ird >> 12;
    let sz = (ird >> 6) & 3;
    // Special instructions first.
    match ird {
        0x4E73 | 0x4E75 | 0x4E77 => return base, // RTE, RTS, RTR
        0x4E58..=0x4E5F => return base.wrapping_add(2), // UNLK
        _ => {}
    }
    if ird & 0xFFC0 == 0x4EC0 {
        return base; // JMP
    }
    if ird & 0xFFC0 == 0x4E80 {
        return base.wrapping_add(2 * words(mode, reg)); // JSR
    }
    if hi == 6 {
        // BSR stacks the target, a taken Bcc the address after its opcode.
        return if ird >> 8 == 0x61 { addr } else { base };
    }
    if hi == 5 && ird & 0xF8 == 0xC8 {
        return base.wrapping_add(2); // DBcc
    }
    if ird & 0xFB80 == 0x4880 {
        // MOVEM: the register mask is one more extension word.
        return base.wrapping_add(4 + 2 * words(mode, reg));
    }
    if matches!(ird & 0xF138, 0xD108 | 0x9108 | 0xB108) && sz != 3 {
        return base.wrapping_add(2); // ADDX, SUBX, CMPM
    }
    let long = match hi {
        1..=3 => hi == 2,
        0 | 5 => sz == 2,
        4 => matches!(ird & 0xFF00, 0x4000 | 0x4200 | 0x4400 | 0x4600 | 0x4A00) && sz == 2,
        9 | 0xB | 0xD => (ird >> 6) & 7 == 7 || sz == 2,
        8 | 0xC => sz == 2,
        _ => false,
    };
    // Extra offset of a read through one effective address.
    let single = |m: u8, r: u8| -> u32 {
        match (m, r) {
            (4, _) if !long => 2,
            (7, 0) => 2,
            (7, 1) => 4,
            _ => 0,
        }
    };
    if (1..=3).contains(&hi) {
        if !write {
            return base.wrapping_add(single(mode, reg));
        }
        let dm = ((ird >> 6) & 7) as u8;
        let dr = ((ird >> 9) & 7) as u8;
        let sw = match (mode, reg) {
            (7, 4) if long => 2,
            (7, 4) => 1,
            _ => words(mode, reg),
        };
        let abs_long_dst = dm == 7 && dr == 1 && mode < 2;
        return base.wrapping_add(2 + 2 * sw + if abs_long_dst { 2 } else { 0 });
    }
    // Immediate operand of ORI, ANDI, SUBI, ADDI, EORI, CMPI.
    let imm = if hi == 0 && matches!(ird & 0xF00, 0x000 | 0x200 | 0x400 | 0x600 | 0xA00 | 0xC00) {
        if long {
            2
        } else {
            1
        }
    } else {
        0
    };
    base.wrapping_add(single(mode, reg) + 2 * imm)
}

/// Interrupt plan (44 clocks): the last `np` is appended below so the plan stays static.
pub fn interrupt_plan() -> Plan {
    &INTERRUPT_FULL
}

static INTERRUPT_FULL: [Tok; 18] = {
    let mut t = [Tok::N(0); 18];
    let mut i = 0;
    while i < 17 {
        t[i] = INTERRUPT[i];
        i += 1;
    }
    t[17] = Tok::Pf;
    t
};

/// Reset plan (40 clocks): `n(14) nF nf nV nv np n np`.
pub fn reset_plan() -> Plan {
    &RESET
}

/// True when `k` is one of the exception hooks.
pub fn is_hook(k: u8) -> bool {
    (HK_PRIV..=HK_PC_SET).contains(&k)
}

fn enter_supervisor(cpu: &mut Cpu) {
    cpu.lat.aux = cpu.regs.pc.wrapping_sub(4);
    cpu.lat.aux2 = u32::from(cpu.regs.sr);
    cpu.regs.sr = (cpu.regs.sr | S) & !T;
}

/// Semantics of the exception hooks. The instruction at IR is at `PC - 4` when the plan starts.
pub fn hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_PRIV => enter_supervisor(cpu),
        HK_SET_PRIV => cpu.lat.vec = VEC_PRIVILEGE,
        HK_VEC_INSN => {
            let ird = cpu.lat.ird;
            cpu.lat.vec = match ird {
                0x4E40..=0x4E4F => 32 + (ird & 15) as u8,
                0x4E76 => 7,
                _ => 6,
            };
        }
        HK_PC_NEXT => cpu.lat.aux = cpu.lat.aux.wrapping_add(2),
        HK_SUP_EARLY => enter_supervisor(cpu),
        HK_PC_SET => cpu.lat.aux = cpu.regs.pc.wrapping_sub(4),
        HK_INT_START => {
            enter_supervisor(cpu);
            cpu.regs.sr = (cpu.regs.sr & !0x0700) | (u16::from(cpu.lat.level) << 8);
        }
        HK_PUSH_PC_LO => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(2);
            cpu.lat.val = cpu.lat.aux & 0xFFFF;
        }
        HK_PUSH_SR => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(6);
            cpu.lat.val = cpu.lat.aux2;
        }
        HK_PUSH_PC_HI => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(4);
            cpu.lat.val = cpu.lat.aux >> 16;
            cpu.regs.ssp = cpu.regs.ssp.wrapping_sub(6);
        }
        HK_IACK => cpu.lat.ea = 0x00FF_FFF1 | (u32::from(cpu.lat.level) << 1),
        HK_VEC_ACK => cpu.lat.vec = cpu.lat.val as u8,
        HK_VECTOR => cpu.lat.ea = u32::from(cpu.lat.vec) * 4,
        HK_JUMP => cpu.regs.pc = cpu.lat.val,
        HK_RST_BEGIN => {
            cpu.regs.sr = 0x2700;
            cpu.lat.ea = 0;
        }
        HK_RST_SSP => {
            cpu.regs.ssp = cpu.lat.val;
            cpu.lat.ea = 4;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::state::CpuState;

    struct Ram {
        mem: std::vec::Vec<(u32, u16)>,
        writes: std::vec::Vec<(u32, u16)>,
        reads: std::vec::Vec<(u32, u8)>,
        ipl: u8,
        ack: Option<u8>,
        acks: std::vec::Vec<u8>,
        fcs: std::vec::Vec<u8>,
    }

    impl Bus for Ram {
        fn read(&mut self, a: u32, fc: u8, _u: bool, _l: bool) -> u16 {
            self.reads.push((a, fc));
            self.mem
                .iter()
                .find(|(x, _)| *x == a)
                .map_or(0, |(_, d)| *d)
        }
        fn write(&mut self, a: u32, _fc: u8, _u: bool, _l: bool, d: u16) {
            self.writes.push((a, d));
        }
        fn ipl(&mut self) -> u8 {
            self.ipl
        }
        fn on_tick(&mut self, pins: crate::pins::Pins) {
            if pins.as_() {
                self.fcs.push(pins.fc());
            }
        }
        fn int_ack(&mut self, level: u8) -> Option<u8> {
            self.acks.push(level);
            self.ack
        }
    }

    fn ram(mem: &[(u32, u16)]) -> Ram {
        Ram {
            mem: mem.to_vec(),
            writes: std::vec::Vec::new(),
            reads: std::vec::Vec::new(),
            ipl: 0,
            ack: None,
            acks: std::vec::Vec::new(),
            fcs: std::vec::Vec::new(),
        }
    }

    /// CPU at PC = 0x20 (opcode at 0x1C), SSP 0x800, NOP in IR.
    fn cpu(sr: u16) -> Cpu {
        let mut c = Cpu::new();
        c.load_state(&CpuState {
            a: [0; 7],
            d: [0; 8],
            usp: 0x400,
            ssp: 0x800,
            sr,
            pc: 0x20,
            prefetch: [0x4E71, 0x4E71],
        });
        c
    }

    #[test]
    fn e11_02_frame_layout() {
        let mut r = ram(&[(0x10, 0x0000), (0x12, 0x1000)]);
        let mut c = cpu(0x2015);
        c.exception(4);
        c.step(&mut r);
        assert_eq!(c.regs.ssp, 0x7FA);
        assert_eq!(
            r.writes,
            std::vec![(0x7FE, 0x1C), (0x7FA, 0x2015), (0x7FC, 0)]
        );
    }

    #[test]
    fn e11_02_super_mode() {
        let mut r = ram(&[(0x10, 0x0000), (0x12, 0x1000)]);
        let mut c = cpu(0x8015);
        c.exception(4);
        c.step(&mut r);
        assert_eq!(c.regs.sr, 0x2015);
        assert_eq!(r.writes[1], (0x7FA, 0x8015));
    }

    #[test]
    fn e11_02_vector_address() {
        let mut r = ram(&[(0x10, 0x0000), (0x12, 0x1000)]);
        let mut c = cpu(0x2000);
        c.exception(4);
        c.step(&mut r);
        assert!(r.reads.contains(&(0x10, 5)) && r.reads.contains(&(0x12, 5)));
        assert_eq!(c.regs.pc, 0x1004);
        let mut r = ram(&[(0xFC, 0x0000), (0xFE, 0x2000)]);
        let mut c = cpu(0x2000);
        c.exception(63);
        c.step(&mut r);
        assert_eq!(c.regs.pc, 0x2004);
    }

    #[test]
    fn e11_02_clocks() {
        let mut r = ram(&[]);
        let mut c = cpu(0x2000);
        c.exception(4);
        assert_eq!(c.step(&mut r), 34);
    }

    fn irq(level: u8, sr: u16, ack: Option<u8>, mem: &[(u32, u16)]) -> (Cpu, Ram, u32) {
        let mut r = ram(mem);
        r.ipl = level;
        r.ack = ack;
        let mut c = cpu(sr);
        let n = c.step(&mut r);
        (c, r, n)
    }

    #[test]
    fn e11_05_neogeo_levels() {
        for (level, vec) in [(1u8, 25u32), (2, 26), (3, 27)] {
            let (c, r, n) = irq(level, 0x2000, None, &[(vec * 4, 0), (vec * 4 + 2, 0x3000)]);
            assert_eq!((n, c.regs.pc), (44, 0x3004), "level {level}");
            assert_eq!(
                r.writes,
                std::vec![(0x7FE, 0x1C), (0x7FA, 0x2000), (0x7FC, 0)]
            );
        }
    }

    #[test]
    fn e11_05_host_vector() {
        let (c, _r, n) = irq(2, 0x2000, Some(0x40), &[(0x100, 0), (0x102, 0x4000)]);
        assert_eq!((n, c.regs.pc), (44, 0x4004));
    }

    #[test]
    fn e11_05_level_masked() {
        for (level, sr) in [(3u8, 0x2300u16), (2, 0x2300), (1, 0x2100)] {
            let (c, _r, n) = irq(level, sr, None, &[]);
            assert_eq!((n, c.regs.pc), (4, 0x22), "level {level}");
        }
    }

    #[test]
    fn e11_05_level_taken() {
        let (c, r, n) = irq(4, 0x2300, None, &[(0x70, 0), (0x72, 0x3000)]);
        assert_eq!(n, 44);
        assert_eq!(c.regs.sr & 0x0700, 0x0400);
        assert_eq!(c.regs.sr & 0x2000, 0x2000);
        assert_eq!(r.writes[1], (0x7FA, 0x2300));
        // user mode: S set, T cleared, SSP used
        let (c, _r, _n) = irq(1, 0x8000, None, &[]);
        assert_eq!(c.regs.sr & 0xA000, 0x2000);
        assert_eq!(c.regs.ssp, 0x7FA);
    }

    #[test]
    fn e11_05_level7_nmi() {
        let (c, _r, n) = irq(7, 0x2700, None, &[(0x7C, 0), (0x7E, 0x3000)]);
        assert_eq!((n, c.regs.pc), (44, 0x3004));
        assert_eq!(c.regs.sr & 0x0700, 0x0700);
    }

    #[test]
    fn e11_05_ack_fc7() {
        let (_c, r, _n) = irq(5, 0x2000, None, &[]);
        assert_eq!(r.acks, std::vec![5]);
        assert_eq!(r.fcs.iter().filter(|fc| **fc == 7).count(), 3);
    }

    #[test]
    fn e11_05_autovector() {
        let (_c, r, _n) = irq(6, 0x2000, None, &[]);
        assert!(r.reads.contains(&(30 * 4, 5)));
    }

    #[test]
    fn e11_06_reset_vectors() {
        let mut r = ram(&[(0, 0), (2, 0x0800), (4, 0), (6, 0x2000)]);
        let mut c = cpu(0x0000);
        c.reset();
        let n = c.step(&mut r);
        assert_eq!((n, c.regs.ssp, c.regs.pc), (40, 0x800, 0x2004));
    }

    #[test]
    fn e11_06_reset_state() {
        let mut r = ram(&[]);
        let mut c = cpu(0x8004);
        c.reset();
        c.step(&mut r);
        assert_eq!(c.regs.sr, 0x2700);
    }

    /// CPU at PC = 0x20 (opcode `op` at 0x1C), A0 = `a0`, SSP 0x800.
    fn ae_cpu(op: u16, a0: u32, sr: u16, ssp: u32) -> Cpu {
        let mut c = Cpu::new();
        let mut a = [0u32; 7];
        a[0] = a0;
        c.load_state(&CpuState {
            a,
            d: [0x1111_2222; 8],
            usp: 0x400,
            ssp,
            sr,
            pc: 0x20,
            prefetch: [op, 0x4E71],
        });
        c
    }

    const VEC3: [(u32, u16); 2] = [(0xC, 0), (0xE, 0x3000)];

    #[test]
    fn e11_03_odd_word_read() {
        // MOVE.w (A0),D1 with A0 odd: the read is aborted, vector 3 is taken.
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x3210, 0x1001, 0x2000, 0x800);
        let n = c.step(&mut r);
        assert_eq!(c.regs.pc, 0x3004);
        assert_eq!(n, 58);
        assert!(!r.reads.iter().any(|(a, _)| *a == 0x1000), "no data cycle");
        assert_eq!(c.regs.d[1], 0x1111_2222, "no commit");
    }

    #[test]
    fn e11_03_frame_14_bytes() {
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x3210, 0x1001, 0x2000, 0x800);
        c.step(&mut r);
        assert_eq!(c.regs.ssp, 0x800 - 14);
        assert_eq!(
            r.writes,
            std::vec![
                (0x7FE, 0x1E),   // PC low
                (0x7FA, 0x2000), // SR
                (0x7FC, 0),      // PC high
                (0x7F8, 0x3210), // IR
                (0x7F6, 0x1001), // access address low
                (0x7F2, 0x3215), // status: read, supervisor data
                (0x7F4, 0),      // access address high
            ]
        );
    }

    #[test]
    fn e11_03_status_word() {
        // Write access in user mode: R/W = 0, FC = 1, the old SR is stacked, S set and T cleared.
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x3081, 0x1001, 0x8010, 0x800);
        c.regs.usp = 0x400;
        c.step(&mut r);
        assert_eq!(c.regs.sr, 0x2010);
        let status = r.writes.iter().find(|(a, _)| *a == 0x7F2).unwrap().1;
        assert_eq!(status, 0x3081 & 0xFFE0 | 1);
        assert_eq!(r.writes[1], (0x7FA, 0x8010));
    }

    #[test]
    fn e11_03_no_commit_write() {
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x3081, 0x1001, 0x2000, 0x800);
        c.step(&mut r);
        // Seven stack writes only: nothing reached the faulting address.
        assert_eq!(r.writes.len(), 7);
        assert!(r.writes.iter().all(|(a, _)| (0x7F2..0x800).contains(a)));
    }

    #[test]
    fn e11_03_odd_pc_fetch() {
        // JMP (A0) to an odd address: the fetch of the target is the aborted cycle.
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x4ED0, 0x1001, 0x2000, 0x800);
        c.step(&mut r);
        assert_eq!(c.regs.pc, 0x3004);
        assert_eq!(r.writes.len(), 7);
        assert_eq!(r.writes[4], (0x7F6, 0x1001));
    }

    #[test]
    fn e11_03_double_fault() {
        // Odd SSP: the frame itself faults, the CPU halts.
        let mut r = ram(&VEC3);
        let mut c = ae_cpu(0x3210, 0x1001, 0x2000, 0x801);
        c.step(&mut r);
        assert!(c.is_halted());
    }

    /// Vector table: every vector in `vecs` leads to 0x3000.
    fn vtable(vecs: &[u32]) -> Ram {
        let mut m = std::vec::Vec::new();
        for v in vecs {
            m.push((v * 4, 0));
            m.push((v * 4 + 2, 0x3000));
        }
        ram(&m)
    }

    /// CPU with `op` in IR, `ext` in IRC, SR = `sr`, SSP 0x800.
    fn op_cpu(op: u16, ext: u16, sr: u16) -> Cpu {
        let mut c = ae_cpu(op, 0, sr, 0x800);
        c.regs.irc = ext;
        c
    }

    /// Clocks of one instruction and the PC reached.
    fn run1(c: &mut Cpu, r: &mut Ram) -> (u32, u32) {
        let n = c.step(r);
        (n, c.regs.pc)
    }

    #[test]
    fn e11_04_illegal_vector4() {
        let mut r = vtable(&[4]);
        let mut c = op_cpu(0x4AFC, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r), (34, 0x3004));
    }

    #[test]
    fn e11_04_line_a_vector10() {
        let mut r = vtable(&[10]);
        let mut c = op_cpu(0xA000, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r), (34, 0x3004));
    }

    #[test]
    fn e11_04_line_f_vector11() {
        let mut r = vtable(&[11]);
        let mut c = op_cpu(0xF000, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r), (34, 0x3004));
    }

    #[test]
    fn e11_04_privilege_vector8() {
        // RTE, STOP and RESET in user mode.
        for (op, ext) in [(0x4E73u16, 0x4E71u16), (0x4E72, 0x2700), (0x4E70, 0x4E71)] {
            let mut r = vtable(&[8]);
            let mut c = op_cpu(op, ext, 0x0000);
            assert_eq!(run1(&mut c, &mut r), (34, 0x3004), "{op:#06x}");
            assert_eq!(c.regs.sr & 0x2000, 0x2000);
        }
    }

    #[test]
    fn e11_07_decode_legal() {
        // TRAP #0..#15 take vectors 32..47.
        for n in 0..16u16 {
            let mut r = vtable(&[32 + u32::from(n)]);
            let mut c = op_cpu(0x4E40 | n, 0x4E71, 0x2000);
            assert_eq!(run1(&mut c, &mut r).1, 0x3004, "TRAP #{n}");
        }
    }

    #[test]
    fn e11_07_decode_illegal() {
        // 0x4E50 is LINK, not TRAP: no vector 32+.
        let mut r = vtable(&[4]);
        let mut c = op_cpu(0x4E50, 0x0000, 0x2000);
        assert_ne!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_07_plan_basic() {
        let mut r = vtable(&[33]);
        let mut c = op_cpu(0x4E41, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).0, 34);
    }

    #[test]
    fn e11_07_flags_sample() {
        // TRAP stacks SR and the PC after the opcode; T is cleared, S set, flags kept.
        let mut r = vtable(&[33]);
        let mut c = op_cpu(0x4E41, 0x4E71, 0x801F);
        run1(&mut c, &mut r);
        assert_eq!(c.regs.sr, 0x201F);
        assert_eq!(r.writes[1], (0x7FA, 0x801F));
        assert_eq!(r.writes[0], (0x7FE, 0x1E));
    }

    #[test]
    fn e11_08_decode_legal() {
        let mut r = vtable(&[7]);
        let mut c = op_cpu(0x4E76, 0x4E71, 0x2002);
        assert_eq!(run1(&mut c, &mut r), (34, 0x3004));
    }

    #[test]
    fn e11_08_decode_illegal() {
        // 0x4E77 is RTR: no trap even with V set.
        let mut r = vtable(&[7]);
        let mut c = op_cpu(0x4E77, 0x4E71, 0x2002);
        c.regs.a[6] = 0;
        assert_ne!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_08_plan_basic() {
        // V clear: 4 clocks.
        let mut r = vtable(&[7]);
        let mut c = op_cpu(0x4E76, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).0, 4);
    }

    #[test]
    fn e11_08_flags_sample() {
        let mut r = vtable(&[7]);
        let mut c = op_cpu(0x4E76, 0x4E71, 0x0003);
        run1(&mut c, &mut r);
        assert_eq!(c.regs.sr, 0x2003);
    }

    #[test]
    fn e11_09_decode_legal() {
        // CHK D1,D0: D0 above the bound takes vector 6.
        let mut r = vtable(&[6]);
        let mut c = op_cpu(0x4181, 0x4E71, 0x2000);
        c.regs.d[0] = 20;
        c.regs.d[1] = 10;
        assert_eq!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_09_decode_illegal() {
        // 0x4180 with an odd size field (LEA/CHK long are illegal on the 68000): 0x41C0 is LEA.
        let mut r = vtable(&[6]);
        let mut c = op_cpu(0x41C0, 0x4E71, 0x2000);
        assert_ne!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_09_plan_basic() {
        // In range: 10 clocks; D0 > bound: 38; D0 < 0: 40.
        let cases = [(5u32, 10u32, 10u32), (20, 10, 38), (0xFFFF_FFFF, 10, 40)];
        for (d0, d1, clocks) in cases {
            let mut r = vtable(&[6]);
            let mut c = op_cpu(0x4181, 0x4E71, 0x2000);
            c.regs.d[0] = d0;
            c.regs.d[1] = d1;
            assert_eq!(run1(&mut c, &mut r).0, clocks, "D0={d0:#x}");
        }
    }

    #[test]
    fn e11_09_flags_sample() {
        // D0 above the bound clears N; D0 negative sets it.
        for (d0, n) in [(20u32, 0u16), (0xFFFF_FFFF, 8)] {
            let mut r = vtable(&[6]);
            let mut c = op_cpu(0x4181, 0x4E71, 0x2000);
            c.regs.d[0] = d0;
            c.regs.d[1] = 10;
            run1(&mut c, &mut r);
            assert_eq!(c.regs.sr & 8, n, "D0={d0:#x}");
        }
    }

    #[test]
    fn e11_10_decode_legal() {
        for op in [0xA000u16, 0xA123, 0xAFFF] {
            let mut r = vtable(&[10]);
            let mut c = op_cpu(op, 0x4E71, 0x2000);
            assert_eq!(run1(&mut c, &mut r).1, 0x3004);
        }
    }

    #[test]
    fn e11_10_decode_illegal() {
        let mut r = vtable(&[10]);
        let mut c = op_cpu(0xB000, 0x4E71, 0x2000);
        assert_ne!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_10_plan_basic() {
        let mut r = vtable(&[10]);
        let mut c = op_cpu(0xA000, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).0, 34);
    }

    #[test]
    fn e11_10_flags_sample() {
        // The stacked PC is the address of the line A opcode itself.
        let mut r = vtable(&[10]);
        let mut c = op_cpu(0xA000, 0x4E71, 0x2015);
        run1(&mut c, &mut r);
        assert_eq!(c.regs.sr, 0x2015);
        assert_eq!(r.writes[0], (0x7FE, 0x1C));
    }

    #[test]
    fn e11_11_decode_legal() {
        for op in [0xF000u16, 0xF123, 0xFFFF] {
            let mut r = vtable(&[11]);
            let mut c = op_cpu(op, 0x4E71, 0x2000);
            assert_eq!(run1(&mut c, &mut r).1, 0x3004);
        }
    }

    #[test]
    fn e11_11_decode_illegal() {
        let mut r = vtable(&[11]);
        let mut c = op_cpu(0xE000, 0x4E71, 0x2000);
        assert_ne!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_11_plan_basic() {
        let mut r = vtable(&[11]);
        let mut c = op_cpu(0xF000, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).0, 34);
    }

    #[test]
    fn e11_11_flags_sample() {
        let mut r = vtable(&[11]);
        let mut c = op_cpu(0xF000, 0x4E71, 0x2015);
        run1(&mut c, &mut r);
        assert_eq!(c.regs.sr, 0x2015);
        assert_eq!(r.writes[0], (0x7FE, 0x1C));
    }

    /// Supervisor stack holding SR and PC for an RTE.
    fn rte_ram(sr: u16, pc: u32) -> Ram {
        ram(&[
            (0x800, sr),
            (0x802, (pc >> 16) as u16),
            (0x804, pc as u16),
            (0x3000, 0x4E71),
            (0x3002, 0x4E71),
        ])
    }

    #[test]
    fn e11_12_decode_legal() {
        let mut r = rte_ram(0x0004, 0x3000);
        let mut c = op_cpu(0x4E73, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).1, 0x3004);
    }

    #[test]
    fn e11_12_decode_illegal() {
        // 0x4E74 is RTD (68010): illegal on the 68000.
        let mut r = vtable(&[4]);
        let mut c = op_cpu(0x4E74, 0x0000, 0x2000);
        assert_eq!(run1(&mut c, &mut r), (34, 0x3004));
    }

    #[test]
    fn e11_12_plan_basic() {
        let mut r = rte_ram(0x0004, 0x3000);
        let mut c = op_cpu(0x4E73, 0x4E71, 0x2000);
        assert_eq!(run1(&mut c, &mut r).0, 20);
    }

    #[test]
    fn e11_12_flags_sample() {
        // SR comes back from the stack (user mode), the stack pointer moves by 6.
        let mut r = rte_ram(0x0015, 0x3000);
        let mut c = op_cpu(0x4E73, 0x4E71, 0x2000);
        run1(&mut c, &mut r);
        assert_eq!(c.regs.sr, 0x0015);
        assert_eq!(c.regs.ssp, 0x806);
    }

    #[test]
    fn e11_13_decode_legal() {
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E72, 0x2000, 0x2700);
        c.step(&mut r);
        assert!(c.is_stopped());
    }

    #[test]
    fn e11_13_decode_illegal() {
        // 0x4E71 is NOP: it does not stop.
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E71, 0x4E71, 0x2700);
        c.step(&mut r);
        assert!(!c.is_stopped());
    }

    #[test]
    fn e11_13_plan_basic() {
        // STOP takes 4 clocks, then the CPU idles 4 clocks per step.
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E72, 0x2000, 0x2700);
        assert_eq!(c.step(&mut r), 4);
        assert_eq!(c.step(&mut r), 4);
        assert!(c.is_stopped());
    }

    #[test]
    fn e11_13_flags_sample() {
        // The immediate word replaces SR.
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E72, 0x2015, 0x2700);
        c.step(&mut r);
        assert_eq!(c.regs.sr, 0x2015);
    }

    #[test]
    fn e11_14_decode_legal() {
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E70, 0x4E71, 0x2000);
        assert_eq!(c.step(&mut r), 132);
    }

    #[test]
    fn e11_14_decode_illegal() {
        // 0x4E71 is NOP, not RESET.
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E71, 0x4E71, 0x2000);
        assert_eq!(c.step(&mut r), 4);
    }

    #[test]
    fn e11_14_plan_basic() {
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E70, 0x4E71, 0x2000);
        assert_eq!(c.step(&mut r), 132);
    }

    #[test]
    fn e11_14_flags_sample() {
        // RESET asserts the reset line only: registers and flags are untouched.
        let mut r = vtable(&[]);
        let mut c = op_cpu(0x4E70, 0x4E71, 0x201F);
        c.step(&mut r);
        assert_eq!(c.regs.sr, 0x201F);
    }

    #[test]
    fn e11_06_double_fault_halt() {
        let mut r = ram(&[]);
        let mut c = cpu(0x2000);
        assert!(!c.bus_fault());
        c.exception(4);
        c.step_clocks(&mut r, 10);
        assert!(c.bus_fault());
        assert!(c.is_halted());
        assert_eq!(c.step(&mut r), 0);
        c.reset();
        assert!(!c.is_halted());
    }
}
