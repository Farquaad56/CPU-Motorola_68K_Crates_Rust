//! OR (`0x8000 ddd o ss ea`) and ORI (`0x0000 | ss << 6 | ea` + immediate): bitwise OR.
//! N and Z from the result, V and C cleared, X untouched. The SST files `OR.*` also hold ORI.
//! Timing: docs/specs/instr/OR.md; plans and semantics are shared in `crate::alu`.

use crate::alu::{and_rules, arith_hook, logic_plan, Alu, N_RULES_AND};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of OR.
pub const ID: InsnId = InsnId(31);

static RULES_ARR: [Rule; N_RULES_AND] = and_rules(0x8000, 0x0000, ID);

/// Decoding rules of OR.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    logic_plan(cpu)
}

/// Semantics of the hooks of OR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    arith_hook(Alu::Or, k, cpu);
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
    fn e08_02_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * 53 * 3 + 8 * 3 * 42 + 3 * 50);
    }

    #[test]
    fn e08_02_decode_illegal() {
        for op in [0x8008, 0x80C0, 0x8100, 0x8140] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_02_plan_basic() {
        assert_eq!(clocks(0x8000 | 0x41), 4); // .W D1,D0
        assert_eq!(clocks(0x8000 | 0x81), 8); // .L D1,D0
        assert_eq!(clocks(0x8000 | 0x90), 14); // .L (A0),D0
        assert_eq!(clocks(0x8000 | 0x150), 12); // .W D0,(A0)
        assert_eq!(clocks(0x0000), 8); // I.B #,D0
        assert_eq!(clocks(0x0080), 16); // I.L #,D0
    }

    #[test]
    fn e08_02_flags_sample() {
        let (cpu, n) = run(0x8001, 0x0F, 0xF0, X | V | C); // OR.B D1,D0
        assert_eq!(
            (n, cpu.regs.d[0] & 0xFF, cpu.regs.sr & 0x1F),
            (4, 0xFF, X | N)
        );
        let (cpu, _) = run(0x8041, 0x1234_0000, 0, N); // OR.W: zero
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x1234_0000, Z));
        let (cpu, _) = run(0x8081, 1, 2, 0); // OR.L
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (3, 0));
        let _ = (C, N, V, X, Z);
    }
}
