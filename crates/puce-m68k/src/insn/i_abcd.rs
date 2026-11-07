//! ABCD (`0xc100 | Dx << 9 | m << 3 | Dy`): packed decimal byte addition with extend.
//! N, Z (only cleared), V (undefined, as Musashi), X = C = decimal carry. Timing: docs/specs/instr/ABCD.md.

use crate::alu::{bcd_hook, bcd_plan, Bcd};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of ABCD.
pub const ID: InsnId = InsnId(46);

static RULES_ARR: [Rule; 2] = [
    Rule {
        mask: 0xF1F8,
        value: 0xc100,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xF1F8,
        value: 0xc108,
        id: ID,
        variant: 1,
    },
];

/// Decoding rules of ABCD.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    bcd_plan(cpu)
}

/// Semantics of the hooks of ABCD.
pub fn hook(k: u8, cpu: &mut Cpu) {
    bcd_hook(Bcd::Abcd, k, cpu);
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
    fn e08_17_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 128);
    }

    #[test]
    fn e08_17_decode_illegal() {
        for op in [0xc140, 0xc148, 0xc110] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_17_plan_basic() {
        assert_eq!(clocks(0xc101), 6); // Dy,Dx
        assert_eq!(clocks(0xc109), 18); // -(Ay),-(Ax)
    }

    #[test]
    fn e08_17_flags_sample() {
        let (cpu, n) = run(0xc101, 0x19, 0x28, 0x0);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x47, 0x0));
        let (cpu, n) = run(0xc101, 0x99, 0x1, 0x4);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x0, 0x15));
        let (cpu, n) = run(0xc101, 0x0, 0x0, 0x10);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x1, 0x0));
        let (cpu, _) = run(0xc101, 0x1234_5600, 0x00, 0x1F);
        assert_eq!(cpu.regs.d[0] >> 8, 0x12_3456);
    }
}
