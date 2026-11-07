//! LEA (`0100 aaa1 11 ea`): loads the effective address into An; no flags. Timing:
//! docs/specs/instr/LEA.md (4 / 8 / 12 clocks). Variant = addressing form: 0 (An), 1 d16(An),
//! 2 d8(An,Xn), 3 abs.W, 4 abs.L, 5 d16(PC), 6 d8(PC,Xn).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    run_ea_hook, Size, HK_EA_ABSL_HI, HK_EA_ABSL_LO, HK_EA_ABSW, HK_EA_AN, HK_EA_DISP, HK_EA_INDEX,
    HK_EA_PCDISP, HK_EA_PCINDEX,
};
use crate::engine::plan::{Plan, Tok};

/// Identifier of LEA.
pub const ID: InsnId = InsnId(7);

const HK_SET: u8 = 0x60; // An = ea
const HK_SET_NOP: u8 = 0x61; // placeholder (no effect)

/// Decoding rules.
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xF1F8,
        value: 0x41D0,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xF1F8,
        value: 0x41E8,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xF1F8,
        value: 0x41F0,
        id: ID,
        variant: 2,
    },
    Rule {
        mask: 0xF1FF,
        value: 0x41F8,
        id: ID,
        variant: 3,
    },
    Rule {
        mask: 0xF1FF,
        value: 0x41F9,
        id: ID,
        variant: 4,
    },
    Rule {
        mask: 0xF1FF,
        value: 0x41FA,
        id: ID,
        variant: 5,
    },
    Rule {
        mask: 0xF1FF,
        value: 0x41FB,
        id: ID,
        variant: 6,
    },
];

static P0: &[Tok] = &[Tok::H(HK_EA_AN), Tok::H(HK_SET), Tok::Pf];
static P1: &[Tok] = &[Tok::H(HK_EA_DISP), Tok::H(HK_SET), Tok::Pf, Tok::Pf];
static P2: &[Tok] = &[
    Tok::H(HK_EA_INDEX),
    Tok::H(HK_SET),
    Tok::N(2),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];
static P3: &[Tok] = &[Tok::H(HK_EA_ABSW), Tok::H(HK_SET), Tok::Pf, Tok::Pf];
static P4: &[Tok] = &[
    Tok::H(HK_EA_ABSL_HI),
    Tok::H(HK_SET_NOP),
    Tok::Pf,
    Tok::H(HK_EA_ABSL_LO),
    Tok::H(HK_SET),
    Tok::Pf,
    Tok::Pf,
];
static P5: &[Tok] = &[Tok::H(HK_EA_PCDISP), Tok::H(HK_SET), Tok::Pf, Tok::Pf];
static P6: &[Tok] = &[
    Tok::H(HK_EA_PCINDEX),
    Tok::H(HK_SET),
    Tok::N(2),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

/// Plan of LEA for an addressing form.
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

/// Semantics of the hooks of LEA.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    match k {
        HK_SET => {
            let a = ((ird >> 9) & 7) as u8;
            let v = cpu.lat.ea;
            cpu.regs.set_an(a, v);
        }
        HK_SET_NOP => {}
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

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, ext: u16, st: CpuState) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, ext];
        cpu.load_state(&st);
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    #[test]
    fn e06_07_decode_legal() {
        // 8 address registers x (3 modes x 8 + 4 absolute/PC forms)
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 * 28);
        assert_eq!(decode(0x41D0).variant, 0);
        assert_eq!(decode(0x43F9).variant, 4);
    }

    #[test]
    fn e06_07_decode_illegal() {
        // Dn, An, (An)+, -(An), immediate are not control addressing modes
        for op in [0x41C0u16, 0x41C8, 0x41D8, 0x41E0, 0x41FC] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_07_plan_basic() {
        let c: std::vec::Vec<u32> = (0..7).map(|v| plan_clocks(plan(v))).collect();
        assert_eq!(c, std::vec![4, 8, 12, 8, 12, 8, 12]);
    }

    #[test]
    fn e06_07_flags_sample() {
        let st = CpuState {
            a: [0x1000, 0, 0, 0, 0, 0, 0],
            sr: 0x271F,
            pc: 0x20,
            ..CpuState::default()
        };
        // LEA (A0),A1: flags untouched
        let (cpu, n) = run(0x43D0, 0, st);
        assert_eq!((n, cpu.regs.sr), (4, 0x271F));
        assert_eq!(cpu.regs.an(1), 0x1000);
        // LEA -2(A0),A2
        let (cpu, n) = run(0x45E8, 0xFFFE, st);
        assert_eq!((n, cpu.regs.an(2)), (8, 0x0FFE));
        // LEA $12345678.L,A3
        let (cpu, n) = run(0x47F9, 0x1234, st);
        assert_eq!(n, 12);
        let _ = cpu;
    }
}
