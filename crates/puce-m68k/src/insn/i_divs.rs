//! DIVS.w (`1000 ddd 111 ea`): `Dn = (remainder << 16) | quotient` of `Dn / source.w`. N and Z from
//! the quotient, V and C cleared, X untouched; on overflow V and N are set and Dn is unchanged.
//! Division by zero is an exception (E11, not yet implemented: the operands are left alone).
//! The duration depends on the operands (`crate::alu::divs_clocks`). Timing: docs/specs/instr/DIVS.md.

use crate::alu::{div_hook, div_plan, mul_rules, N_RULES_MUL};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of DIVS.
pub const ID: InsnId = InsnId(29);

static RULES_ARR: [Rule; N_RULES_MUL] = mul_rules(0x81C0, ID);

/// Decoding rules of DIVS.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operand: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    div_plan(cpu)
}

/// Semantics of the hooks of DIVS.
pub fn hook(k: u8, cpu: &mut Cpu) {
    div_hook(true, k, cpu);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::regs::{C, N, V, X, Z};
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
    fn e07_15a_decode_legal() {
        // per Dn: Dn (8), 5 memory modes (40), 5 absolute / PC / immediate forms
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * (8 + 40 + 5));
    }

    #[test]
    fn e07_15a_decode_illegal() {
        for op in [0x81C0 | 0x08, 0x80C0 | 0x01] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_15a_plan_basic() {
        // static duration: np only, the variable idle cycle counts for 0
        assert_eq!(clocks(0x81C0 | 0x01), 4);
        assert_eq!(clocks(0x81C0 | 0x10), 8);
    }

    #[test]
    fn e07_15a_flags_sample() {
        // -100 / 7 = -14 remainder -2
        let (cpu, _) = run(0x81C1, 0xFFFF_FF9C, 7, X | V | C);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0xFFFE_FFF2, X | N));
        // 5 / 7 = 0 remainder 5: Z
        let (cpu, _) = run(0x81C1, 5, 7, 0);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x0005_0000, Z));
        // 100 / -7 = -14 remainder 2
        let (cpu, _) = run(0x81C1, 100, 0xFFF9, 0);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x0002_FFF2, N));
    }

    #[test]
    fn e07_15b_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 * 53);
        assert_eq!(decode(0x81C0 | 0x3C).id, ID);
    }

    #[test]
    fn e07_15b_decode_illegal() {
        for op in [0x81C0 | 0x09, 0x80C0 | 0x3C] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_15b_plan_basic() {
        assert_eq!(clocks(0x81C0 | 0x39), 16); // abs.L: np np nr + np
    }

    #[test]
    fn e07_15b_flags_sample() {
        // overflow: 16 clocks (18 for a negative dividend), operands unchanged
        let (cpu, n) = run(0x81C1, 0x0001_0000, 1, 0);
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (16, 0x0001_0000, V | N)
        );
        let (_, n) = run(0x81C1, 0xFFFE_FFFF, 1, 0);
        assert_eq!(n, 18);
        // 100 / 7: 6 + 55 - 1 + 12 zero bits among the 15 high bits of the quotient = 72 -> 144
        let (_, n) = run(0x81C1, 100, 7, 0);
        assert_eq!(n, 144);
        let _ = (C, X);
    }
}
