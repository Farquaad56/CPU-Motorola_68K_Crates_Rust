//! ROXR.b / .w / .l: shift or rotate of a data register by a count (immediate 1-8 or in Dn modulo 64)
//! or of a memory word by 1. Flags per the manual. Timing: docs/specs/instr/ASL.md (variable idle
//! cycle `Tok::Nv`, 2 clocks per step). Shared semantics and plans live in `crate::alu`.

use crate::alu::{shift_hook, shift_plan, shift_rules, Shift, N_RULES_SHIFT};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of ROXR.
pub const ID: InsnId = InsnId(41);

static RULES_ARR: [Rule; N_RULES_SHIFT] = shift_rules(2, false, ID);

/// Decoding rules of ROXR.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its form: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    shift_plan(cpu)
}

/// Semantics of the hooks of ROXR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    shift_hook(Shift::Roxr, k, cpu);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::state::CpuState;

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, d0: u32, d1: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, d1, 0, 0, 0, 0, 0, 0],
            sr,
            pc: 0x20,
            prefetch: [op, 0],
            ..CpuState::default()
        });
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    fn clocks(op: u16) -> u32 {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        plan_clocks(plan(0, &cpu))
    }

    #[test]
    fn e08_12_decode_legal() {
        // register forms: 3 sizes x (count/Dn) x 8 x 8; memory: 5 modes x 8 + abs.W + abs.L
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 3 * 2 * 8 * 8 + 5 * 8 + 2);
    }

    #[test]
    fn e08_12_decode_illegal() {
        for op in [0xe4c0, 0xe4c8, 0xe4fa, 0xe4fc, 0xe310, 0xe208] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_12_plan_basic() {
        assert_eq!(clocks(0xe210), 6); // .B #,D0 (idle cycles are added at run time)
        assert_eq!(clocks(0xe290), 8); // .L #,D0
        assert_eq!(clocks(0xe4d0), 12); // (A0)
        assert_eq!(clocks(0xe4f9), 20); // abs.L
    }

    #[test]
    fn e08_12_flags_sample() {
        let (cpu, n) = run(0xe210, 0x81, 0x0, 0x0);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (8, 0x40, 0x11));
        let (cpu, n) = run(0xe270, 0x12348001, 0x3, 0x10);
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (12, 0x12347000, 0x0)
        );
        let (cpu, n) = run(0xe090, 0x80ff0001, 0x0, 0x11);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (24, 0x380ff00, 0x0));
    }
}
