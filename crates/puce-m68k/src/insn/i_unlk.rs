//! UNLK An (`0100 1110 0101 1rrr`): SP = An, An = (SP)+; 12 clocks (`nU nu np`), no flags.
//! Timing: docs/specs/instr/UNLINK.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::AddrSrc;
use crate::engine::plan::{Plan, Tok};

/// Identifier of UNLK.
pub const ID: InsnId = InsnId(13);

const HK_LOAD: u8 = 0x68; // ea = An
const HK_FIN: u8 = 0x69; // SP = ea + 4, An = val

/// Decoding rules.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFF8,
    value: 0x4E58,
    id: ID,
    variant: 0,
}];

/// Plan: `nU nu np`.
pub fn plan(_variant: u8) -> Plan {
    &[
        Tok::H(HK_LOAD),
        Tok::Rl(AddrSrc::Ea),
        Tok::H(HK_FIN),
        Tok::Pf,
    ]
}

/// Semantics of the hooks of UNLK.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let r = (cpu.lat.ird & 7) as u8;
    if k == HK_LOAD {
        cpu.lat.ea = cpu.regs.an(r);
    } else {
        let sp = cpu.lat.ea.wrapping_add(4);
        cpu.regs.set_an(7, sp);
        let v = cpu.lat.val;
        cpu.regs.set_an(r, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;

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
    fn e06_13_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8);
        assert_eq!(decode(0x4E5E).id, ID);
    }

    #[test]
    fn e06_13_decode_illegal() {
        for op in [0x4E50u16, 0x4E60, 0x4E68] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_13_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 12);
    }

    #[test]
    fn e06_13_flags_sample() {
        let mut r = ram();
        r.w[0x1FE >> 1] = 0x0000;
        r.w[0x200 >> 1] = 0x00AB;
        r.w[0x202 >> 1] = 0xCDEF;
        let st = CpuState {
            a: [0, 0, 0, 0, 0, 0, 0x200],
            usp: 0x10,
            sr: 0x001F,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4E5E, 0, st, &mut r); // UNLK A6
        assert_eq!(n, 12);
        assert_eq!(cpu.regs.sr, 0x001F);
        assert_eq!(cpu.regs.an(6), 0x00AB_CDEF);
        assert_eq!(cpu.regs.usp, 0x204);
    }
}
