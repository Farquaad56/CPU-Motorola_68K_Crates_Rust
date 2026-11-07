//! NOP (`0100 1110 0111 0001`): no operation, 4 clocks, one prefetch (`np`).
//! Timing: docs/specs/instr/NOP.md (Yacht.txt lines 851-856). Flags are not affected.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};

/// Identifier of NOP.
pub const ID: InsnId = InsnId(1);

/// Decoding rules: the single encoding `0x4E71`.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFFF,
    value: 0x4E71,
    id: ID,
    variant: 0,
}];

/// Plan: the prefetch of the next opcode is the only bus cycle.
pub fn plan(_variant: u8) -> Plan {
    &[Tok::Pf]
}

/// NOP has no hook.
pub fn hook(_k: u8, _cpu: &mut Cpu) {}

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
        fn read(&mut self, _a: u32, _fc: u8, _u: bool, _l: bool) -> u16 {
            0x4E71
        }
        fn write(&mut self, _a: u32, _fc: u8, _u: bool, _l: bool, _d: u16) {}
    }

    #[test]
    fn e06_01_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 1);
        assert_eq!(decode(0x4E71).id, ID);
    }

    #[test]
    fn e06_01_decode_illegal() {
        // neighbours: RESET (0x4E70), STOP (0x4E72), RTE (0x4E73)
        for op in [0x4E70u16, 0x4E72, 0x4E73, 0x4E75, 0x4E61, 0x4C71] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_01_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 4);
        assert_eq!(plan(0), &[Tok::Pf]);
    }

    #[test]
    fn e06_01_flags_sample() {
        // NOP leaves X N Z V C untouched, whatever their value
        for ccr in [0u16, X | N | Z | V | C, Z | C] {
            let mut cpu = Cpu::new();
            cpu.load_state(&CpuState {
                sr: 0x2700 | ccr,
                pc: 0x1000,
                prefetch: [0x4E71, 0x4E71],
                ..CpuState::default()
            });
            let n = cpu.step(&mut Ram);
            assert_eq!(n, 4);
            assert_eq!(cpu.regs.sr, 0x2700 | ccr);
            assert_eq!(cpu.regs.pc, 0x1002);
        }
    }
}
