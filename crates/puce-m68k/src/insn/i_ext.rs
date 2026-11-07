//! EXT.w / EXT.l (`0100 100o oo00 0rrr`, opmode 010 / 011): sign extension of Dn, 4 clocks (`np`).
//! N and Z from the result, V and C cleared, X untouched. Timing: docs/specs/instr/EXT.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};
use crate::regs::{C, N, V, Z};

/// Identifier of EXT.
pub const ID: InsnId = InsnId(11);

/// Decoding rules: variant 0 = byte to word, 1 = word to long.
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xFFF8,
        value: 0x4880,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xFFF8,
        value: 0x48C0,
        id: ID,
        variant: 1,
    },
];

/// Plan: `np`.
pub fn plan(_variant: u8) -> Plan {
    &[Tok::H(1), Tok::Pf]
}

/// Hook 1: extend and set the flags.
pub fn hook(_k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let r = (ird & 7) as usize;
    let d = cpu.regs.d[r];
    let (v, neg, zero) = if ird & 0x40 == 0 {
        let w = d as u8 as i8 as i16 as u16;
        cpu.regs.d[r] = (d & 0xFFFF_0000) | w as u32;
        (w, w & 0x8000 != 0, w == 0)
    } else {
        let l = d as u16 as i16 as i32 as u32;
        cpu.regs.d[r] = l;
        ((l >> 16) as u16, l & 0x8000_0000 != 0, l == 0)
    };
    let _ = v;
    let mut sr = cpu.regs.sr & !(N | Z | V | C);
    if zero {
        sr |= Z;
    }
    if neg {
        sr |= N;
    }
    cpu.regs.sr = sr;
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
    fn e06_11_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 16);
        assert_eq!(decode(0x4881).variant, 0);
        assert_eq!(decode(0x48C1).variant, 1);
    }

    #[test]
    fn e06_11_decode_illegal() {
        for op in [0x4888u16, 0x48C8, 0x49C0, 0x4880 | 0x10] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_11_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 4);
        assert_eq!(plan_clocks(plan(1)), 4);
    }

    #[test]
    fn e06_11_flags_sample() {
        let mut r = ram();
        let mk = |d: u32, sr: u16| CpuState {
            d: [d, 0, 0, 0, 0, 0, 0, 0],
            sr,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4880, 0, mk(0x1234_5680, V | C), &mut r); // EXT.W
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (4, 0x1234_FF80, N));
        let (cpu, _) = run(0x48C0, 0, mk(0xABCD_0000, X), &mut r); // EXT.L
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0, X | Z));
        let (cpu, _) = run(0x48C0, 0, mk(0x0001_7FFF, 0), &mut r);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (0x7FFF, 0));
    }
}
