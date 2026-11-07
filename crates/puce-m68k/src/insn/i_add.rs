//! ADD (`0xD000 ddd o ss ea`): `<ea>,Dn` and `Dn,<ea>`, byte / word / long. X N Z V C set as for
//! the arithmetic operation. Timing: docs/specs/instr/ADD.md ("ADD, SUB" table); plans and
//! semantics are shared in `crate::alu`.

use crate::alu::{arith_hook, arith_plan, arith_rules, Alu, N_RULES_ARITH};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of ADD.
pub const ID: InsnId = InsnId(15);

static RULES_ARR: [Rule; N_RULES_ARITH] = arith_rules(0xD000, 0x0600, 0x5000, ID);

/// Decoding rules of ADD.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    arith_plan(cpu)
}

/// Semantics of the hooks of ADD.
pub fn hook(k: u8, cpu: &mut Cpu) {
    arith_hook(Alu::Add, k, cpu);
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
    fn e07_01_decode_legal() {
        // per Dn: <ea>,Dn byte 8+40+5, word and long 8+8+40+5; Dn,<ea> 3 x (40+2)
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * (53 + 2 * 61 + 3 * 42) + 3 * 50 + 8 * (50 + 2 * 58));
    }

    #[test]
    fn e07_01_decode_illegal() {
        // byte from An, ADDA/SUBA slot (opmode 3), X variants (Dn,<ea> with mode 0/1)
        for op in [0xD000 | 0x08, 0xD000 | 0xC0, 0xD000 | 0x140, 0xD000 | 0x148] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_01_plan_basic() {
        assert_eq!(clocks(0xD000 | 0x41), 4); // .W D1,D0
        assert_eq!(clocks(0xD000 | 0x81), 8); // .L D1,D0
        assert_eq!(clocks(0xD000 | 0x90), 14); // .L (A0),D0
        assert_eq!(clocks(0xD000 | 0x150), 12); // .W D0,(A0)
        assert_eq!(clocks(0x0600), 8); // I.B #,D0
        assert_eq!(clocks(0x0600 | 0x90), 28); // I.L #,(A0)
        assert_eq!(clocks(0x5000 | 0x48), 8); // Q.W #,A0
        assert_eq!(clocks(0x5000 | 0x80), 8); // Q.L #,D0
    }

    #[test]
    fn e07_01_flags_sample() {
        let (cpu, n) = run(0xD001, 0x7F, 1, X | C); // ADD.B D1,D0
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (4, 0x80, N | V));
        let (cpu, _) = run(0xD001, 0x1234_56FF, 1, 0); // carry out
        assert_eq!(
            (cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (0x1234_5600, X | C | Z)
        );
        let (cpu, n) = run(0xD081, 0x7FFF_FFFF, 1, 0); // ADD.L D1,D0
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.sr & 0x1F),
            (8, 0x8000_0000, N | V)
        );
    }
}
