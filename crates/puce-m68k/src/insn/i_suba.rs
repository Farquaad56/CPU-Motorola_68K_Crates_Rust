//! SUBA.w / SUBA.l (`1001 aaa s11 ea`): `An -= source` (word sign-extended), no flags.
//! Timing: docs/specs/instr/ADDA.md (ADDA, SUBA table).

use crate::alu::{adda_hook, adda_plan, adda_rules, Alu, N_RULES_ADDA};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of SUBA.
pub const ID: InsnId = InsnId(18);

static RULES_ARR: [Rule; N_RULES_ADDA] = adda_rules(0x9000, ID);

/// Decoding rules of SUBA.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    adda_plan(cpu)
}

/// Semantics of the hooks of SUBA.
pub fn hook(k: u8, cpu: &mut Cpu) {
    adda_hook(Alu::Sub, k, cpu);
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

    fn run(op: u16, d0: u32, d1: u32, a0: u32, a1: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, d1, 0, 0, 0, 0, 0, 0],
            a: [a0, a1, 0, 0, 0, 0, 0],
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
    fn e07_05_decode_legal() {
        // per An: modes Dn, An (8 each), 5 memory modes (8 each), 5 absolute / PC / immediate forms
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * 2 * (8 + 8 + 40 + 5));
    }

    #[test]
    fn e07_05_decode_illegal() {
        // opmode 0..2 are ADD/SUB, opmode 4..6 with mode 0/1 are the X forms
        for op in [0x9000 | 0x41, 0x9000 | 0x108, 0x9000 | 0x140] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_05_plan_basic() {
        assert_eq!(clocks(0x9000 | 0xC1), 8); // .W D1,A0
        assert_eq!(clocks(0x9000 | 0x1C1), 8); // .L D1,A0
        assert_eq!(clocks(0x9000 | 0x1D1), 14); // .L (A1),A0
        assert_eq!(clocks(0x9000 | 0xD1), 12); // .W (A1),A0
    }

    #[test]
    fn e07_05_flags_sample() {
        let sr = 0x271F;
        // word source is sign-extended; flags untouched
        let (cpu, n) = run(0x9000 | 0xC1, 0, 0xFFFF, 0x1000, 0, sr);
        assert_eq!(
            (n, cpu.regs.an(0), cpu.regs.sr),
            (8, 0x1000u32.wrapping_sub(0xFFFF_FFFF), sr)
        );
        let (cpu, _) = run(0x9000 | 0x1C1, 0, 0x0001_0002, 0x1000, 0, 0);
        assert_eq!(cpu.regs.an(0), 0x1000u32.wrapping_sub(0x0001_0002));
        let (cpu, _) = run(0x9000 | 0x1C9, 0, 0, 0x10, 0x20, N | Z);
        assert_eq!(
            (cpu.regs.an(0), cpu.regs.sr & 0x1F),
            (0x10u32.wrapping_sub(0x20), N | Z)
        );
        let _ = (C, V, X);
    }
}
