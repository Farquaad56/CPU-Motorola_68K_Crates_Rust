//! System instructions: TRAP, TRAPV, RTE, STOP, RESET and CHK (variants of one module).
//!
//! Timing: oracle SingleStepTests/m68000 (TRAP 34, TRAPV 4 or 34, RTE 20, STOP 4, RESET 132,
//! CHK 10 without trap, 38 (Dn > bound) or 40 (Dn < 0) with).

use crate::alu::{chk_hook, chk_plan, mode_rules};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::exception;
use crate::regs::{S, V};

/// Identifier of the system instructions.
pub const ID: InsnId = InsnId(69);

const V_TRAP: u8 = 0;
const V_TRAPV: u8 = 1;
const V_RTE: u8 = 2;
const V_STOP: u8 = 3;
const V_RESET: u8 = 4;
const V_CHK: u8 = 5;

const HK_EA: u8 = 0x20; // ea = SP
const HK_PC: u8 = 0x21; // aux = SR word, ea = SP + 2
const HK_POP: u8 = 0x22; // SP += 6, SR, PC
const HK_STOP: u8 = 0x23; // SR = immediate, stopped

const fn build_rules() -> [Rule; 16] {
    let none = Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    };
    let mut out = [none; 16];
    out[0] = Rule {
        mask: 0xFFF0,
        value: 0x4E40,
        id: ID,
        variant: V_TRAP,
    };
    out[1] = Rule {
        mask: 0xFFFF,
        value: 0x4E76,
        id: ID,
        variant: V_TRAPV,
    };
    out[2] = Rule {
        mask: 0xFFFF,
        value: 0x4E73,
        id: ID,
        variant: V_RTE,
    };
    out[3] = Rule {
        mask: 0xFFFF,
        value: 0x4E72,
        id: ID,
        variant: V_STOP,
    };
    out[4] = Rule {
        mask: 0xFFFF,
        value: 0x4E70,
        id: ID,
        variant: V_RESET,
    };
    // CHK <ea>,Dn: the data register number (bits 9-11) is free.
    let chk = mode_rules::<11>(0x4180, ID);
    let mut i = 0;
    while i < 11 {
        out[5 + i] = Rule {
            mask: chk[i].mask & !0x0E00,
            value: chk[i].value,
            id: ID,
            variant: V_CHK,
        };
        i += 1;
    }
    out
}

static RULES_ARR: [Rule; 16] = build_rules();

/// Decoding rules of the system instructions.
pub static RULES: &[Rule] = &RULES_ARR;

static NO_TRAP: [Tok; 1] = [Tok::Pf];

static RTE: [Tok; 7] = [
    Tok::H(HK_EA),
    Tok::Rw(AddrSrc::Ea),
    Tok::H(HK_PC),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_POP),
    Tok::Pf,
    Tok::Pf,
];

static STOP: [Tok; 2] = [Tok::H(HK_STOP), Tok::N(4)];

static RESET: [Tok; 2] = [Tok::N(128), Tok::Pf];

/// Plan of the instruction: the privileged ones raise a privilege violation in user mode.
pub fn plan(variant: u8, cpu: &Cpu) -> Plan {
    let user = cpu.regs.sr & S == 0;
    match variant {
        V_TRAP => exception::trap_plan(),
        V_TRAPV => {
            if cpu.regs.sr & V != 0 {
                exception::trapv_plan()
            } else {
                &NO_TRAP
            }
        }
        V_CHK => chk_plan(cpu),
        _ if user => exception::privilege_violation(),
        V_RTE => &RTE,
        V_STOP => &STOP,
        _ => &RESET,
    }
}

/// Semantics of the hooks of the system instructions.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    if ird & 0xF1C0 == 0x4180 {
        chk_hook(k, cpu);
        return;
    }
    match k {
        HK_EA => cpu.lat.ea = cpu.regs.an(7),
        HK_PC => {
            cpu.lat.aux = cpu.lat.val;
            cpu.lat.ea = cpu.regs.an(7).wrapping_add(2);
        }
        HK_POP => {
            let sp = cpu.regs.an(7).wrapping_add(6);
            cpu.regs.set_an(7, sp);
            cpu.regs.pc = cpu.lat.val;
            cpu.regs.set_sr(cpu.lat.aux as u16);
        }
        HK_STOP => {
            let sr = cpu.regs.irc;
            cpu.regs.set_sr(sr);
            cpu.stopped = true;
        }
        _ if exception::is_hook(k) => exception::hook(k, cpu),
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
    }
    impl Bus for Ram {
        fn read(&mut self, a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            self.mem
                .iter()
                .find(|(x, _)| *x == a)
                .map_or(0, |(_, d)| *d)
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, ext: u16, sr: u16, d: [u32; 8]) -> (Cpu, u32) {
        let mut r = Ram {
            mem: std::vec![(0x18, 0), (0x1C, 0x0000), (0x1E, 0x1000), (0x80, 0), (0x82, 0x2000)],
        };
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [0; 7],
            d,
            usp: 0x400,
            ssp: 0x800,
            sr,
            pc: 0x20,
            prefetch: [op, ext],
        });
        let n = cpu.step(&mut r);
        (cpu, n)
    }

    #[test]
    fn e15_02_trap_34_clocks() {
        let (cpu, n) = run(0x4E40, 0, 0x2000, [0; 8]);
        assert_eq!(n, 34);
        assert_eq!(cpu.lat.vec, 32);
    }

    #[test]
    fn e15_02_trapv_clear_4_set_34() {
        assert_eq!(run(0x4E76, 0, 0x2000, [0; 8]).1, 4);
        let (cpu, n) = run(0x4E76, 0, 0x2002, [0; 8]);
        assert_eq!(n, 34);
        assert_eq!(cpu.lat.vec, 7);
    }

    #[test]
    fn e15_02_chk_paths() {
        // CHK D1,D0 (0x4181): D1 is the bound, D0 the value
        let mut d = [0u32; 8];
        d[1] = 10;
        d[0] = 5;
        assert_eq!(run(0x4181, 0, 0x2000, d).1, 10);
        d[0] = 11;
        assert_eq!(run(0x4181, 0, 0x2000, d).1, 38);
        d[0] = 0xFFFF;
        let (cpu, n) = run(0x4181, 0, 0x2000, d);
        assert_eq!(n, 40);
        assert_ne!(cpu.regs.sr & crate::regs::N, 0);
    }

    #[test]
    fn e15_02_privileged_in_user_mode() {
        for op in [0x4E73u16, 0x4E72, 0x4E70] {
            let (cpu, n) = run(op, 0x2000, 0x0000, [0; 8]);
            assert_eq!(n, 34, "{op:04x}");
            assert_eq!(cpu.lat.vec, 8);
        }
    }

    #[test]
    fn e15_02_reset_132_stop_4() {
        assert_eq!(run(0x4E70, 0, 0x2000, [0; 8]).1, 132);
        let (cpu, n) = run(0x4E72, 0x2700, 0x2000, [0; 8]);
        assert_eq!(n, 4);
        assert!(cpu.stopped);
        assert_eq!(cpu.regs.sr, 0x2700);
    }
}
