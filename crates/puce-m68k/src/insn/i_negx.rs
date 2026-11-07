//! NEGX.b / .w / .l (`0100 0000 ss ea`): `dst = 0 - dst - X`. Z is only cleared, never set.
//! Timing: docs/specs/instr/CLR.md ("CLR, NEGX, NEG, NOT").

use crate::alu::{unary_hook, unary_plan, unary_rules, Unary, N_RULES_UNARY};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of NEGX.
pub const ID: InsnId = InsnId(22);

static RULES_ARR: [Rule; N_RULES_UNARY] = unary_rules(0x4000, ID);

/// Decoding rules of NEGX.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operand: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    unary_plan(cpu, false)
}

/// Semantics of the hooks of NEGX.
pub fn hook(k: u8, cpu: &mut Cpu) {
    unary_hook(Unary::Negx, k, cpu);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::regs::{C, N, X, Z};
    use crate::state::CpuState;

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, d0: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, 0, 0, 0, 0, 0, 0, 0],
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
    fn e07_08_decode_legal() {
        // per size: Dn (8), 5 memory modes (40), abs.W and abs.L
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 3 * (8 + 40 + 2));
        assert_eq!(decode(0x4000).id, ID);
        assert_eq!(decode(0x4000 | 0x80 | 0x39).id, ID);
    }

    #[test]
    fn e07_08_decode_illegal() {
        // An, PC-relative and immediate modes; size 3 is another instruction
        for op in [0x4000 | 0x08, 0x4000 | 0x3A, 0x4000 | 0x3C, 0x4000 | 0xC0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_08_plan_basic() {
        assert_eq!(clocks(0x4000), 4); // .B D0
        assert_eq!(clocks(0x4000 | 0x80), 6); // .L D0
        assert_eq!(clocks(0x4000 | 0x10), 12); // .B (A0)
        assert_eq!(clocks(0x4000 | 0x90), 20); // .L (A0)
    }

    #[test]
    fn e07_08_flags_sample() {
        let (cpu, n) = run(0x4000, 0, X | Z); // NEGX.B D0 with X
        assert_eq!(
            (n, cpu.regs.d[0] & 0xFF, cpu.regs.sr & 0x1F),
            (4, 0xFF, X | N | C)
        );
        let (cpu, _) = run(0x4000, 0, Z); // no X: zero result keeps Z
        assert_eq!((cpu.regs.d[0] & 0xFF, cpu.regs.sr & 0x1F), (0, Z));
        let (cpu, _) = run(0x4080, 0x8000_0000, X); // NEGX.L: 0 - 0x80000000 - 1
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x7FFF_FFFF, X | C));
    }
}
