//! NBCD (`0100 1000 00 ea`): negate decimal with extend, byte. Flags as for SBCD from zero.
//! Timing: docs/specs/instr/NBCD.md.

use crate::alu::{nbcd_hook, nbcd_plan, nbcd_rules, N_RULES_NBCD};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of NBCD.
pub const ID: InsnId = InsnId(48);

static RULES_ARR: [Rule; N_RULES_NBCD] = nbcd_rules(ID);

/// Decoding rules of NBCD.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    nbcd_plan(cpu)
}

/// Semantics of the hooks of NBCD.
pub fn hook(k: u8, cpu: &mut Cpu) {
    nbcd_hook(k, cpu);
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
    fn e08_19_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 50);
    }

    #[test]
    fn e08_19_decode_illegal() {
        for op in [0x4808, 0x483A, 0x483C, 0x4840] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_19_plan_basic() {
        assert_eq!(clocks(0x4800), 6); // Dn
        assert_eq!(clocks(0x4810), 12); // (A0)
        assert_eq!(clocks(0x4839), 20); // (xxx).L
    }

    #[test]
    fn e08_19_flags_sample() {
        let (cpu, n) = run(0x4800, 0x12345601, 0, 0x0);
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (6, 0x12345699, 0x19)
        );
        let (cpu, n) = run(0x4800, 0x12345600, 0, 0x10);
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (6, 0x12345699, 0x19)
        );
        let (cpu, n) = run(0x4800, 0x12345600, 0, 0x4);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x12345600, 0x4));
        let (cpu, n) = run(0x4800, 0x12345655, 0, 0x0);
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (6, 0x12345645, 0x13)
        );
    }
}
