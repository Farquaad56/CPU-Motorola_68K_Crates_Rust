//! CMPA.w / CMPA.l (`1011 aaa s11 ea`): `An - source` flags on 32 bits (word sign-extended), X
//! untouched. Timing: docs/specs/instr/CMPA.md.

use crate::alu::{adda_rules, cmpa_hook, cmpa_plan, N_RULES_ADDA};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of CMPA.
pub const ID: InsnId = InsnId(25);

static RULES_ARR: [Rule; N_RULES_ADDA] = adda_rules(0xB000, ID);

/// Decoding rules of CMPA.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operand: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    cmpa_plan(cpu)
}

/// Semantics of the hooks of CMPA.
pub fn hook(k: u8, cpu: &mut Cpu) {
    cmpa_hook(k, cpu);
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

    fn run(op: u16, d0: u32, d1: u32, a0: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, d1, 0, 0, 0, 0, 0, 0],
            a: [a0, 0, 0, 0, 0, 0, 0],
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
    fn e07_10_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * 2 * (8 + 8 + 40 + 5));
    }

    #[test]
    fn e07_10_decode_illegal() {
        for op in [0xB041u16, 0xB108, 0xB140] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_10_plan_basic() {
        assert_eq!(clocks(0xB0C1), 6); // CMPA.W D1,A0
        assert_eq!(clocks(0xB1D1), 14); // CMPA.L (A1),A0
        assert_eq!(clocks(0xB0D1), 10); // CMPA.W (A1),A0
    }

    #[test]
    fn e07_10_flags_sample() {
        // word source sign-extended: 0x1000 - 0xFFFFFFFF borrows
        let (cpu, n) = run(0xB0C1, 0, 0xFFFF, 0x1000, 0);
        assert_eq!((n, cpu.regs.an(0), cpu.regs.sr & 0x1F), (6, 0x1000, C));
        let (cpu, _) = run(0xB1C1, 0, 0x1000, 0x1000, X | N);
        assert_eq!(cpu.regs.sr & 0x1F, X | Z);
        let (cpu, _) = run(0xB0C1, 0, 0x8000, 0, 0);
        assert_eq!(cpu.regs.sr & 0x1F, C);
        let _ = V;
    }
}
