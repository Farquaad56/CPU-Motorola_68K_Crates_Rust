//! ADDX.b / .w / .l (`1101 xxx1 ss 00 m yyy`): `Dx += Dy + X` or `-(Ax) += -(Ay) + X`.
//! Timing: docs/specs/instr/ADDX.md.

use crate::alu::{x_hook, x_plan, x_rules, Alu, N_RULES_X};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of ADDX.
pub const ID: InsnId = InsnId(19);

static RULES_ARR: [Rule; N_RULES_X] = x_rules(0xD000, ID);

/// Decoding rules of ADDX.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    x_plan(cpu)
}

/// Semantics of the hooks of ADDX.
pub fn hook(k: u8, cpu: &mut Cpu) {
    x_hook(Alu::Add, k, cpu);
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
    fn e07_03_decode_legal() {
        // 8 x 8 register pairs x 3 sizes x 2 modes
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * 8 * 3 * 2);
    }

    #[test]
    fn e07_03_decode_illegal() {
        // bits 5-4 must be 00; the ADDA/SUBA slot (opmode 3 / 7) is another instruction
        for op in [0xd000 | 0x110, 0xd000 | 0x1C0, 0xd000 | 0xC0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_03_plan_basic() {
        assert_eq!(clocks(0xd000 | 0x101), 4); // .B D1,D0
        assert_eq!(clocks(0xd000 | 0x181), 8); // .L D1,D0
        assert_eq!(clocks(0xd000 | 0x109), 18); // .B -(A1),-(A0)
        assert_eq!(clocks(0xd000 | 0x189), 30); // .L -(A1),-(A0)
    }

    #[test]
    fn e07_03_flags_sample() {
        // byte: result zero keeps Z set, carry sets X and C
        let (cpu, n) = run(0xd000 | 0x101, 0xFF, 0, 0, 0, X | Z);
        assert_eq!(
            (n, cpu.regs.d[0] & 0xFF, cpu.regs.sr & 0x1F),
            (4, 0x00, X | C | Z)
        );
        // long: overflow
        let (cpu, _) = run(0xd000 | 0x181, 0x7FFF_FFFF, 0, 0, 0, X);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x8000_0000, N | V));
        // non-zero result clears Z
        let (cpu, _) = run(0xd000 | 0x101, 0x10, 0x01, 0, 0, 0);
        assert_eq!((cpu.regs.d[0] & 0xFF, cpu.regs.sr & Z), (0x11, 0));
        let _ = (C, N, V, X);
    }
}
