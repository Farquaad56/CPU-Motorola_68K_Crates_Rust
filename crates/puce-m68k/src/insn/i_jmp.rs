//! JMP (`0100 1110 11 ea`): PC = effective address; no flags. Timing: docs/specs/instr/JMP.md.
//! Variant = addressing form (see `RULES`).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    run_ea_hook, Size, HK_EA_ABSL_HI, HK_EA_ABSL_LO, HK_EA_ABSW, HK_EA_AN, HK_EA_DISP, HK_EA_INDEX,
    HK_EA_PCDISP, HK_EA_PCINDEX,
};
use crate::engine::plan::{Plan, Tok};

/// Identifier of JMP.
pub const ID: InsnId = InsnId(53);

const HK_JUMP: u8 = 0x20; // PC = ea

/// Decoding rules. Variant = addressing form: 0 (An), 1 d16(An), 2 d8(An,Xn), 3 abs.W, 4 abs.L,
/// 5 d16(PC), 6 d8(PC,Xn).
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xfff8,
        value: 0x4ed0,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xfff8,
        value: 0x4ee8,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xfff8,
        value: 0x4ef0,
        id: ID,
        variant: 2,
    },
    Rule {
        mask: 0xffff,
        value: 0x4ef8,
        id: ID,
        variant: 3,
    },
    Rule {
        mask: 0xffff,
        value: 0x4ef9,
        id: ID,
        variant: 4,
    },
    Rule {
        mask: 0xffff,
        value: 0x4efa,
        id: ID,
        variant: 5,
    },
    Rule {
        mask: 0xffff,
        value: 0x4efb,
        id: ID,
        variant: 6,
    },
];

static P0: &[Tok] = &[Tok::H(HK_EA_AN), Tok::H(HK_JUMP), Tok::Pf, Tok::Pf];
static P1: &[Tok] = &[
    Tok::H(HK_EA_DISP),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];
static P2: &[Tok] = &[
    Tok::H(HK_EA_INDEX),
    Tok::N(6),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];
static P3: &[Tok] = &[
    Tok::H(HK_EA_ABSW),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];
static P4: &[Tok] = &[
    Tok::H(HK_EA_ABSL_HI),
    Tok::Pf,
    Tok::H(HK_EA_ABSL_LO),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];
static P5: &[Tok] = &[
    Tok::H(HK_EA_PCDISP),
    Tok::N(2),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];
static P6: &[Tok] = &[
    Tok::H(HK_EA_PCINDEX),
    Tok::N(6),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];

/// Plan of JMP for an addressing form.
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

/// Semantics of the hooks of JMP.
pub fn hook(k: u8, cpu: &mut Cpu) {
    if k == HK_JUMP {
        cpu.regs.pc = cpu.lat.ea;
    } else {
        run_ea_hook(cpu, k, (cpu.lat.ird & 7) as u8, Size::Long);
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
    fn e09_05_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 3 * 8 + 4);
        assert_eq!(decode(0x4ED0).variant, 0);
        assert_eq!(decode(0x4EF9).variant, 4);
    }

    #[test]
    fn e09_05_decode_illegal() {
        for op in [0x4EC0u16, 0x4EC8, 0x4ED8, 0x4EE0, 0x4EFC, 0x4E80] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_05_plan_basic() {
        let c: std::vec::Vec<u32> = (0..7).map(clocks).collect();
        assert_eq!(c, std::vec![8, 10, 14, 10, 12, 10, 14]);
    }

    #[test]
    fn e09_05_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4ED0, 0, 0x1000, 0, 0x15, &mut r);
        assert_eq!((n, cpu.regs.pc, cpu.regs.sr), (8, 0x1004, 0x15));
        let (cpu, n) = run(0x4EE8, 0x0010, 0x1000, 0, 0x15, &mut r);
        assert_eq!((n, cpu.regs.pc), (10, 0x1014));
        let (cpu, n) = run(0x4EF9, 0x0001, 0, 0, 0x15, &mut r);
        assert_eq!((n, cpu.regs.pc), (12, 0x10004));
    }
}
