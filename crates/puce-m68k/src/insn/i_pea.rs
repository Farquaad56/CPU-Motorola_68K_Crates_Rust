//! PEA (`0100 1000 01 ea`): pushes the effective address (long) on the stack; no flags. Timing:
//! docs/specs/instr/PEA.md. Variant = addressing form: 0 (An), 1 d16(An), 2 d8(An,Xn), 3 abs.W,
//! 4 abs.L, 5 d16(PC), 6 d8(PC,Xn).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    run_ea_hook, Size, HK_EA_ABSL_HI, HK_EA_ABSL_LO, HK_EA_ABSW, HK_EA_AN, HK_EA_DISP, HK_EA_INDEX,
    HK_EA_PCDISP, HK_EA_PCINDEX,
};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of PEA.
pub const ID: InsnId = InsnId(8);

const HK_SAVE: u8 = 0x62; // aux = ea
const HK_PUSH: u8 = 0x63; // SP -= 4, ea = SP, val = aux
const HK_SHIFT16: u8 = 0x64;

/// Decoding rules.
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xFFF8,
        value: 0x4850,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xFFF8,
        value: 0x4868,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xFFF8,
        value: 0x4870,
        id: ID,
        variant: 2,
    },
    Rule {
        mask: 0xFFFF,
        value: 0x4878,
        id: ID,
        variant: 3,
    },
    Rule {
        mask: 0xFFFF,
        value: 0x4879,
        id: ID,
        variant: 4,
    },
    Rule {
        mask: 0xFFFF,
        value: 0x487A,
        id: ID,
        variant: 5,
    },
    Rule {
        mask: 0xFFFF,
        value: 0x487B,
        id: ID,
        variant: 6,
    },
];

static P0: &[Tok] = &[
    Tok::H(HK_EA_AN),
    Tok::H(HK_SAVE),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
];
static P1: &[Tok] = &[
    Tok::H(HK_EA_DISP),
    Tok::H(HK_SAVE),
    Tok::Pf,
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
];
static P2: &[Tok] = &[
    Tok::H(HK_EA_INDEX),
    Tok::H(HK_SAVE),
    Tok::N(2),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
];
static P3: &[Tok] = &[
    Tok::H(HK_EA_ABSW),
    Tok::H(HK_SAVE),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P4: &[Tok] = &[
    Tok::H(HK_EA_ABSL_HI),
    Tok::Pf,
    Tok::H(HK_EA_ABSL_LO),
    Tok::H(HK_SAVE),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::Pf,
];
static P5: &[Tok] = &[
    Tok::H(HK_EA_PCDISP),
    Tok::H(HK_SAVE),
    Tok::Pf,
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
];
static P6: &[Tok] = &[
    Tok::H(HK_EA_PCINDEX),
    Tok::H(HK_SAVE),
    Tok::N(2),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
];

/// Plan of PEA for an addressing form.
pub fn plan(variant: u8) -> Plan {
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

/// Semantics of the hooks of PEA.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    match k {
        HK_SAVE => cpu.lat.aux = cpu.lat.ea,
        HK_PUSH => {
            let sp = cpu.regs.an(7).wrapping_sub(4);
            cpu.regs.set_an(7, sp);
            cpu.lat.ea = sp;
            cpu.lat.val = cpu.lat.aux;
        }
        HK_SHIFT16 => cpu.lat.val >>= 16,
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
    }
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, a: u32, _f: u8, _u: bool, _l: bool, d: u16) {
            self.writes.push((a, d));
        }
    }

    #[test]
    fn e06_08_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 3 * 8 + 4);
        assert_eq!(decode(0x4850).variant, 0);
        assert_eq!(decode(0x4879).variant, 4);
    }

    #[test]
    fn e06_08_decode_illegal() {
        for op in [0x4840u16, 0x4848, 0x4858, 0x4860, 0x487C] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_08_plan_basic() {
        let c: std::vec::Vec<u32> = (0..7).map(|v| plan_clocks(plan(v))).collect();
        assert_eq!(c, std::vec![12, 16, 20, 16, 20, 16, 20]);
    }

    #[test]
    fn e06_08_flags_sample() {
        let mut r = Ram {
            writes: std::vec::Vec::new(),
        };
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [0x0012_3456, 0, 0, 0, 0, 0, 0],
            usp: 0x400,
            sr: 0x0015,
            pc: 0x20,
            prefetch: [0x4850, 0],
            ..CpuState::default()
        });
        let n = cpu.step(&mut r);
        assert_eq!(n, 12);
        assert_eq!(cpu.regs.sr, 0x0015);
        assert_eq!(cpu.regs.usp, 0x3FC);
        assert_eq!(r.writes, std::vec![(0x3FC, 0x0012), (0x3FE, 0x3456)]);
    }
}
