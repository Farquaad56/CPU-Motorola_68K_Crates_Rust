//! SWAP (`0100 1000 0100 0rrr`): exchanges the halves of Dn, 4 clocks (`np`). N and Z from the
//! 32-bit result, V and C cleared, X untouched. Timing: docs/specs/instr/SWAP.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};
use crate::regs::{C, N, V, Z};

/// Identifier of SWAP.
pub const ID: InsnId = InsnId(10);

/// Decoding rules.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFF8,
    value: 0x4840,
    id: ID,
    variant: 0,
}];

/// Plan: `np`.
pub fn plan(_variant: u8) -> Plan {
    &[Tok::H(1), Tok::Pf]
}

/// Hook 1: swap the halves and set the flags.
pub fn hook(_k: u8, cpu: &mut Cpu) {
    let r = (cpu.lat.ird & 7) as usize;
    let v = cpu.regs.d[r].rotate_left(16);
    cpu.regs.d[r] = v;
    let mut sr = cpu.regs.sr & !(N | Z | V | C);
    if v == 0 {
        sr |= Z;
    }
    if v & 0x8000_0000 != 0 {
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
    fn e06_10_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8);
        assert_eq!(decode(0x4843).id, ID);
    }

    #[test]
    fn e06_10_decode_illegal() {
        for op in [0x4848u16, 0x4850, 0x4838] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_10_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 4);
    }

    #[test]
    fn e06_10_flags_sample() {
        let mut r = ram();
        let mk = |d: u32, sr: u16| CpuState {
            d: [d, 0, 0, 0, 0, 0, 0, 0],
            sr,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4840, 0, mk(0x1234_8000, X | V | C), &mut r);
        assert_eq!((n, cpu.regs.d[0]), (4, 0x8000_1234));
        assert_eq!(cpu.regs.sr & 0x1F, X | N);
        let (cpu, _) = run(0x4840, 0, mk(0, N), &mut r);
        assert_eq!(cpu.regs.sr & 0x1F, Z);
        let (cpu, _) = run(0x4840, 0, mk(0x0001_0000, Z), &mut r);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr & 0x1F), (1, 0));
    }
}
