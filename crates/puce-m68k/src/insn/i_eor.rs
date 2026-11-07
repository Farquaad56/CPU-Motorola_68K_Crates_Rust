//! EOR (`0xB100 ddd o ss ea`) and EORI (`0x0A00 | ss << 6 | ea` + immediate): bitwise exclusive OR.
//! N and Z from the result, V and C cleared, X untouched. The SST files `EOR.*` also hold EORI.
//! Timing: docs/specs/instr/EOR.md; plans and semantics are shared in `crate::alu`.

use crate::alu::{arith_hook, eor_rules, logic_plan, Alu, N_RULES_EOR};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of EOR.
pub const ID: InsnId = InsnId(32);

static RULES_ARR: [Rule; N_RULES_EOR] = eor_rules(ID);

/// Decoding rules of EOR.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    logic_plan(cpu)
}

/// Semantics of the hooks of EOR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    arith_hook(Alu::Eor, k, cpu);
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
    fn e08_03_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * 3 * (8 + 40 + 2) + 3 * 50);
    }

    #[test]
    fn e08_03_decode_illegal() {
        for op in [0xB000, 0xB108, 0xB0C0, 0xB1C0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_03_plan_basic() {
        assert_eq!(clocks(0xB141), 4); // .W D0,D1
        assert_eq!(clocks(0xB181), 8); // .L D0,D1
        assert_eq!(clocks(0xB190), 20); // .L D0,(A0)
        assert_eq!(clocks(0xB150), 12); // .W D0,(A0)
        assert_eq!(clocks(0x0A00), 8); // I.B #,D0
        assert_eq!(clocks(0x0A80), 16); // I.L #,D0
    }

    #[test]
    fn e08_03_flags_sample() {
        let (cpu, n) = run(0xB300, 0xFF, 0xFF, X | V | C); // EOR.B D1,D0
        assert_eq!((n, cpu.regs.d[0] & 0xFF, cpu.regs.sr & 0x1F), (4, 0, X | Z));
        let (cpu, _) = run(0xB340, 0x00FF, 0x0F0F, 0); // EOR.W D1,D0
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x0FF0, 0));
        let (cpu, n) = run(0xB380, 0xAAAA_AAAA, 0x5555_5555, 0); // EOR.L D1,D0
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (8, 0xFFFF_FFFF, N));
        let _ = (C, N, V, X, Z);
    }
}
