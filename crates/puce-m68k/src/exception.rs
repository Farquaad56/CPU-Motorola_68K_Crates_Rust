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

/// Plan of the privilege violation exception (34 clocks): `nn ns ns ns nV nv np n np`.
pub fn privilege_violation() -> Plan {
    &PRIVILEGE
}

/// Generic exception plan (34 clocks), vector number in `Latch::vec`: `nn ns ns ns nV nv np n np`.
pub fn exception_plan() -> Plan {
    &EXCEPTION
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
    (HK_PRIV..=HK_RST_SSP).contains(&k)
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
