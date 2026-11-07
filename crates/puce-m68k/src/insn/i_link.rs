//! LINK An,#d16 (`0100 1110 0101 0rrr` + d16): pushes An, An = SP, SP += d16; 16 clocks
//! (`np nS ns np`), no flags. Timing: docs/specs/instr/LINK.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::AddrSrc;
use crate::engine::plan::{Plan, Tok};

/// Identifier of LINK.
pub const ID: InsnId = InsnId(12);

const HK_DISP: u8 = 0x65; // aux = sign-extended displacement (IRC), before the Pf consuming it
const HK_PUSH: u8 = 0x66; // val = An, SP -= 4, ea = SP
const HK_FIN: u8 = 0x67; // An = SP; SP += displacement

/// Decoding rules.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFF8,
    value: 0x4E50,
    id: ID,
    variant: 0,
}];

/// Plan: `np nS ns np`.
pub fn plan(_variant: u8) -> Plan {
    &[
        Tok::H(HK_DISP),
        Tok::Pf,
        Tok::H(HK_PUSH),
        Tok::Wl(AddrSrc::Ea),
        Tok::H(HK_FIN),
        Tok::Pf,
    ]
}

/// Semantics of the hooks of LINK.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let r = (cpu.lat.ird & 7) as u8;
    match k {
        HK_DISP => cpu.lat.aux = cpu.regs.irc as i16 as i32 as u32,
        HK_PUSH => {
            cpu.lat.val = cpu.regs.an(r);
            let sp = cpu.regs.an(7).wrapping_sub(4);
            cpu.regs.set_an(7, sp);
            cpu.lat.ea = sp;
        }
        _ => {
            let sp = cpu.regs.an(7);
            cpu.regs.set_an(r, sp);
            cpu.regs.set_an(7, sp.wrapping_add(cpu.lat.aux));
        }
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
    fn e06_12_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8);
        assert_eq!(decode(0x4E56).id, ID);
    }

    #[test]
    fn e06_12_decode_illegal() {
        for op in [0x4E58u16, 0x4E48, 0x4E60] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_12_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 16);
    }

    #[test]
    fn e06_12_flags_sample() {
        let mut r = ram();
        let st = CpuState {
            a: [0, 0, 0, 0, 0, 0, 0x1234_5678],
            usp: 0x400,
            sr: 0x0015,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4E56, 0xFFF0, st, &mut r); // LINK A6,#-16
        assert_eq!(n, 16);
        assert_eq!(cpu.regs.sr, 0x0015);
        assert_eq!(cpu.regs.an(6), 0x3FC);
        assert_eq!(cpu.regs.usp, 0x3FC - 16);
        assert_eq!(r.writes, std::vec![(0x3FC, 0x1234), (0x3FE, 0x5678)]);
    }
}
