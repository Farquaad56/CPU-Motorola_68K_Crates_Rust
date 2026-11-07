//! MULS.w (`1100 ddd 111 ea`): `Dn = Dn.w * source.w` (32-bit result). N and Z from the result,
//! V and C cleared, X untouched. 38 + 2m clocks: the idle cycle after the prefetch lasts 17 + m
//! internal cycles (`Tok::Nv`). Timing: docs/specs/instr/MULU.md.

use crate::alu::{mul_hook, mul_plan, mul_rules, N_RULES_MUL};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of MULS.
pub const ID: InsnId = InsnId(27);

static RULES_ARR: [Rule; N_RULES_MUL] = mul_rules(0xC1C0, ID);

/// Decoding rules of MULS.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operand: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    mul_plan(cpu)
}

/// Semantics of the hooks of MULS.
pub fn hook(k: u8, cpu: &mut Cpu) {
    mul_hook(true, k, cpu);
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
    fn e07_13_decode_legal() {
        // per Dn: Dn (1 form x 8), 5 memory modes (40), 5 absolute / PC / immediate forms
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * (8 + 40 + 5));
    }

    #[test]
    fn e07_13_decode_illegal() {
        // An source, and the other opmode (0xC0C0)
        for op in [0xC1C0 | 0x08, 0xC0C0 | 0x01] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_13_plan_basic() {
        // static duration: the variable idle cycle counts for 0
        assert_eq!(clocks(0xC1C0 | 0x01), 4); // D1,D0
        assert_eq!(clocks(0xC1C0 | 0x10), 8); // (A0),D0
    }

    #[test]
    fn e07_13_flags_sample() {
        // -1 * -1 = 1; 0xFFFF with a zero appended has one 01 pattern: 40 clocks
        let (cpu, n) = run(0xC1C1, 0xFFFF, 0xFFFF, X);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (40, 1, X));
        // 0x5555: 16 patterns, 70 clocks; 0x5555 * 2 = 0xAAAA
        let (cpu, n) = run(0xC1C1, 2, 0x5555, 0);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (70, 0xAAAA, 0));
        // -2 * 3 = -6: N
        let (cpu, _) = run(0xC1C1, 3, 0xFFFE, V | C);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0xFFFF_FFFA, N));
        let _ = Z;
    }
}
