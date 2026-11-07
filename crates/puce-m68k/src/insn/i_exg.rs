//! EXG (`1100 xxx1 oooo oyyy`): exchanges two 32-bit registers, 6 clocks (`np n`), no flags.
//! Variant: 0 Dx,Dy (`01000`), 1 Ax,Ay (`01001`), 2 Dx,Ay (`10001`). Timing: docs/specs/instr/EXG.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};

/// Identifier of EXG.
pub const ID: InsnId = InsnId(9);

/// Decoding rules.
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xF1F8,
        value: 0xC140,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xF1F8,
        value: 0xC148,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xF1F8,
        value: 0xC188,
        id: ID,
        variant: 2,
    },
];

/// Plan: the exchange is done by the hook, then `np n`.
pub fn plan(_variant: u8) -> Plan {
    &[Tok::H(1), Tok::Pf, Tok::N(2)]
}

/// Hook 1: exchange the registers.
pub fn hook(_k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let x = ((ird >> 9) & 7) as usize;
    let y = (ird & 7) as usize;
    match (ird >> 3) & 0x1F {
        0b01000 => cpu.regs.d.swap(x, y),
        0b01001 => {
            let (a, b) = (cpu.regs.an(x as u8), cpu.regs.an(y as u8));
            cpu.regs.set_an(x as u8, b);
            cpu.regs.set_an(y as u8, a);
        }
        _ => {
            let (a, b) = (cpu.regs.d[x], cpu.regs.an(y as u8));
            cpu.regs.d[x] = b;
            cpu.regs.set_an(y as u8, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::regs::{C, N, V, X, Z};
    use crate::state::CpuState;

    struct Ram {
        w: [u16; 0x400],
        writes: std::vec::Vec<(u32, u16)>,
    }
    impl Bus for Ram {
        fn read(&mut self, a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            self.w[(a as usize >> 1) & 0x3FF]
        }
        fn write(&mut self, a: u32, _f: u8, _u: bool, _l: bool, d: u16) {
            self.writes.push((a, d));
            self.w[(a as usize >> 1) & 0x3FF] = d;
        }
    }

    fn ram() -> Ram {
        Ram {
            w: [0; 0x400],
            writes: std::vec::Vec::new(),
        }
    }

    fn run(op: u16, ext: u16, st: CpuState, r: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, ext];
        cpu.load_state(&st);
        let n = cpu.step(r);
        (cpu, n)
    }

    #[test]
    fn e06_09_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 3 * 64);
        assert_eq!(decode(0xC141).variant, 0);
        assert_eq!(decode(0xC149).variant, 1);
        assert_eq!(decode(0xC189).variant, 2);
    }

    #[test]
    fn e06_09_decode_illegal() {
        for op in [0xC150u16, 0xC158, 0xC180, 0xC1C0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_09_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 6);
    }

    #[test]
    fn e06_09_flags_sample() {
        let st = CpuState {
            d: [1, 2, 0, 0, 0, 0, 0, 0],
            a: [0xA0, 0xA1, 0, 0, 0, 0, 0],
            sr: 0x271F,
            pc: 0x20,
            ..CpuState::default()
        };
        let mut r = ram();
        let (cpu, n) = run(0xC141, 0, st, &mut r); // EXG D0,D1
        assert_eq!(
            (n, cpu.regs.d[0], cpu.regs.d[1], cpu.regs.sr),
            (6, 2, 1, 0x271F)
        );
        let (cpu, _) = run(0xC149, 0, st, &mut r); // EXG A0,A1
        assert_eq!((cpu.regs.an(0), cpu.regs.an(1)), (0xA1, 0xA0));
        let (cpu, _) = run(0xC189, 0, st, &mut r); // EXG D0,A1
        assert_eq!((cpu.regs.d[0], cpu.regs.an(1)), (0xA1, 1));
        let _ = (C, N, V, X, Z);
    }
}
