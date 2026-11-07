//! JSR (`0100 1110 10 ea`): pushes the return address (long, high word first) and jumps. No flags.
//! Timing: docs/specs/instr/JSR.md. Variant = addressing form (see `RULES`).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    run_ea_hook, Size, HK_EA_ABSL_HI, HK_EA_ABSL_LO, HK_EA_ABSW, HK_EA_AN, HK_EA_DISP, HK_EA_INDEX,
    HK_EA_PCDISP, HK_EA_PCINDEX,
};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of JSR.
pub const ID: InsnId = InsnId(54);

const HK_JUMP: u8 = 0x20; // PC = ea
const HK_RET: u8 = 0x21; // aux = return address
const HK_PUSH: u8 = 0x22; // SP -= 4, ea = SP, val = aux

/// Decoding rules (same addressing forms as JMP).
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xfff8,
        value: 0x4e90,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xfff8,
        value: 0x4ea8,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xfff8,
        value: 0x4eb0,
        id: ID,
        variant: 2,
    },
    Rule {
        mask: 0xffff,
        value: 0x4eb8,
        id: ID,
        variant: 3,
    },
    Rule {
        mask: 0xffff,
        value: 0x4eb9,
        id: ID,
        variant: 4,
    },
    Rule {
        mask: 0xffff,
        value: 0x4eba,
        id: ID,
        variant: 5,
    },
    Rule {
        mask: 0xffff,
        value: 0x4ebb,
        id: ID,
        variant: 6,
    },
];

static P0: &[Tok] = &[
    Tok::H(HK_EA_AN),
    Tok::H(HK_RET),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P1: &[Tok] = &[
    Tok::H(HK_EA_DISP),
    Tok::H(HK_RET),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P2: &[Tok] = &[
    Tok::H(HK_EA_INDEX),
    Tok::H(HK_RET),
    Tok::N(6),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P3: &[Tok] = &[
    Tok::H(HK_EA_ABSW),
    Tok::H(HK_RET),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P4: &[Tok] = &[
    Tok::H(HK_EA_ABSL_HI),
    Tok::H(HK_RET),
    Tok::Pf,
    Tok::H(HK_EA_ABSL_LO),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P5: &[Tok] = &[
    Tok::H(HK_EA_PCDISP),
    Tok::H(HK_RET),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P6: &[Tok] = &[
    Tok::H(HK_EA_PCINDEX),
    Tok::H(HK_RET),
    Tok::N(6),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];

/// Plan of JSR for an addressing form.
pub fn plan(variant: u8, _cpu: &Cpu) -> Plan {
    match variant {
        0 => P0,
        1 => P1,
        2 => P2,
        3 => P3,
        4 => P4,
        5 => P5,
        _ => P6,
    }
}

/// Semantics of the hooks of JSR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    match k {
        HK_JUMP => cpu.regs.pc = cpu.lat.ea,
        HK_RET => {
            // PC = opcode address + 4 here; the return address follows the extension words
            cpu.lat.aux = match ird & 0x3F {
                0x10..=0x17 => cpu.regs.pc.wrapping_sub(2),
                0x39 => cpu.regs.pc.wrapping_add(2),
                _ => cpu.regs.pc,
            };
        }
        HK_PUSH => {
            let sp = cpu.regs.an(7).wrapping_sub(4);
            cpu.regs.set_an(7, sp);
            cpu.lat.ea = sp;
            cpu.lat.val = cpu.lat.aux;
        }
        _ => {
            run_ea_hook(cpu, k, (ird & 7) as u8, Size::Long);
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
        writes: std::vec::Vec<(u32, u16)>,
        mem: std::vec::Vec<(u32, u16)>,
    }
    impl Bus for Ram {
        fn read(&mut self, a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            self.mem
                .iter()
                .find(|(x, _)| *x == a)
                .map_or(0, |(_, d)| *d)
        }
        fn write(&mut self, a: u32, _f: u8, _u: bool, _l: bool, d: u16) {
            self.writes.push((a, d));
        }
    }

    fn ram(mem: &[(u32, u16)]) -> Ram {
        Ram {
            writes: std::vec::Vec::new(),
            mem: mem.to_vec(),
        }
    }

    fn run(op: u16, ext: u16, a0: u32, d0: u32, sr: u16, r: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [a0, 0, 0, 0, 0, 0, 0],
            d: [d0, 0, 0, 0, 0, 0, 0, 0],
            usp: 0x400,
            sr,
            pc: 0x20,
            prefetch: [op, ext],
            ..CpuState::default()
        });
        let n = cpu.step(r);
        (cpu, n)
    }

    fn clocks(v: u8) -> u32 {
        plan_clocks(plan(v, &Cpu::new()))
    }

    #[test]
    fn e09_06_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 3 * 8 + 4);
        assert_eq!(decode(0x4E90).variant, 0);
        assert_eq!(decode(0x4EB9).variant, 4);
    }

    #[test]
    fn e09_06_decode_illegal() {
        for op in [0x4E80u16, 0x4E88, 0x4E98, 0x4EA0, 0x4EBC, 0x4EC0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_06_plan_basic() {
        let c: std::vec::Vec<u32> = (0..7).map(clocks).collect();
        assert_eq!(c, std::vec![16, 18, 22, 18, 20, 18, 22]);
    }

    #[test]
    fn e09_06_flags_sample() {
        // JSR (A0): return address = opcode + 2 = 0x1E, pushed high word first
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4E90, 0, 0x1000, 0, 0x15, &mut r);
        assert_eq!(
            (n, cpu.regs.pc, cpu.regs.sr, cpu.regs.usp),
            (16, 0x1004, 0x15, 0x3FC)
        );
        assert_eq!(r.writes, std::vec![(0x3FC, 0), (0x3FE, 0x1E)]);
        // JSR d16(A0): return address = opcode + 4 = 0x20
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4EA8, 0x0010, 0x1000, 0, 0x15, &mut r);
        assert_eq!((n, cpu.regs.pc), (18, 0x1014));
        assert_eq!(r.writes, std::vec![(0x3FC, 0), (0x3FE, 0x20)]);
        // JSR (xxx).L: return address = opcode + 6 = 0x22
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4EB9, 0x0001, 0, 0, 0x15, &mut r);
        assert_eq!((n, cpu.regs.pc), (20, 0x10004));
        assert_eq!(r.writes, std::vec![(0x3FC, 0), (0x3FE, 0x22)]);
    }
}
