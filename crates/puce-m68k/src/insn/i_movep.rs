//! MOVEP.w / MOVEP.l (`0000 ddd1 oo00 1aaa` + d16): transfers between Dn and alternate bytes of memory
//! starting at d16(An), most significant byte first. Flags are untouched.
//! `oo`: 00 word memory->Dn, 01 long memory->Dn, 10 word Dn->memory, 11 long Dn->memory.
//! Timing: docs/specs/instr/MOVEP.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{run_ea_hook, Size, HK_EA_DISP};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of MOVEP.
pub const ID: InsnId = InsnId(6);

const HK_START: u8 = 0x58; // aux = 0
const HK_LOADB: u8 = 0x59; // Dn->memory: val = next byte (MSB first)
const HK_ADV: u8 = 0x5A; // ea += 2
const HK_ACC: u8 = 0x5B; // memory->Dn: accumulate val, ea += 2
const HK_FIN: u8 = 0x5C; // memory->Dn: store the accumulated bytes in Dn

/// Decoding rules: variant = `oo`.
pub const RULES: &[Rule] = &[
    Rule {
        mask: 0xF1F8,
        value: 0x0108,
        id: ID,
        variant: 0,
    },
    Rule {
        mask: 0xF1F8,
        value: 0x0148,
        id: ID,
        variant: 1,
    },
    Rule {
        mask: 0xF1F8,
        value: 0x0188,
        id: ID,
        variant: 2,
    },
    Rule {
        mask: 0xF1F8,
        value: 0x01C8,
        id: ID,
        variant: 3,
    },
];

const E: AddrSrc = AddrSrc::Ea;

static P_RW: &[Tok] = &[
    Tok::H(HK_START),
    Tok::H(HK_EA_DISP),
    Tok::Pf,
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::H(HK_FIN),
    Tok::Pf,
];
static P_RL: &[Tok] = &[
    Tok::H(HK_START),
    Tok::H(HK_EA_DISP),
    Tok::Pf,
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::Rb(E),
    Tok::H(HK_ACC),
    Tok::H(HK_FIN),
    Tok::Pf,
];
static P_WW: &[Tok] = &[
    Tok::H(HK_START),
    Tok::H(HK_EA_DISP),
    Tok::Pf,
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::Pf,
];
static P_WL: &[Tok] = &[
    Tok::H(HK_START),
    Tok::H(HK_EA_DISP),
    Tok::Pf,
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::H(HK_LOADB),
    Tok::Wb(E),
    Tok::H(HK_ADV),
    Tok::Pf,
];

/// Plan of MOVEP.
pub fn plan(variant: u8) -> Plan {
    match variant {
        0 => P_RW,
        1 => P_RL,
        2 => P_WW,
        _ => P_WL,
    }
}

/// Semantics of the hooks of MOVEP.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let long = ird & 0x40 != 0;
    let dn = ((ird >> 9) & 7) as usize;
    match k {
        HK_START => cpu.lat.aux = 0,
        HK_LOADB => {
            let n = if long { 4 } else { 2 };
            let i = cpu.lat.aux;
            let shift = 8 * (n - 1 - i);
            cpu.lat.val = (cpu.regs.d[dn] >> shift) & 0xFF;
            cpu.lat.aux = i + 1;
        }
        HK_ADV => cpu.lat.ea = cpu.lat.ea.wrapping_add(2),
        HK_ACC => {
            cpu.lat.aux = (cpu.lat.aux << 8) | (cpu.lat.val & 0xFF);
            cpu.lat.ea = cpu.lat.ea.wrapping_add(2);
        }
        HK_FIN => {
            let v = cpu.lat.aux;
            if long {
                cpu.regs.d[dn] = v;
            } else {
                cpu.regs.d[dn] = (cpu.regs.d[dn] & 0xFFFF_0000) | (v & 0xFFFF);
            }
        }
        _ => {
            run_ea_hook(cpu, k, (ird & 7) as u8, Size::Word);
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
        }
    }

    fn run(op: u16, st: CpuState, ram: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, 0x0000];
        cpu.load_state(&st);
        let n = cpu.step(ram);
        (cpu, n)
    }

    fn ram() -> Ram {
        Ram {
            w: [0; 0x400],
            writes: std::vec::Vec::new(),
        }
    }

    #[test]
    fn e06_06_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 4 * 64);
        assert_eq!(decode(0x0108).variant, 0);
        assert_eq!(decode(0x03C9).variant, 3);
    }

    #[test]
    fn e06_06_decode_illegal() {
        // neighbours: mode 2 (BTST-type slot) and oo with mode 0
        for op in [0x0110u16, 0x0100, 0x0188 | 0x08 << 1] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_06_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 16);
        assert_eq!(plan_clocks(plan(1)), 24);
        assert_eq!(plan_clocks(plan(2)), 16);
        assert_eq!(plan_clocks(plan(3)), 24);
    }

    #[test]
    fn e06_06_flags_sample() {
        // word memory->D0 keeps the upper half and the flags
        let mut r = ram();
        r.w[0x100] = 0xAB00;
        r.w[0x101] = 0xCD00;
        let st = CpuState {
            d: [0x1111_0000, 0, 0, 0, 0, 0, 0, 0],
            a: [0x200, 0, 0, 0, 0, 0, 0],
            sr: 0x271F,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x0108, st, &mut r);
        assert_eq!(n, 16);
        assert_eq!(cpu.regs.d[0], 0x1111_ABCD);
        assert_eq!(cpu.regs.sr, 0x271F);
        // long D0->memory
        let mut r = ram();
        let st = CpuState {
            d: [0x1122_3344, 0, 0, 0, 0, 0, 0, 0],
            a: [0x200, 0, 0, 0, 0, 0, 0],
            sr: 0x2700,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x01C8, st, &mut r);
        assert_eq!(n, 24);
        assert_eq!(cpu.regs.sr, 0x2700);
        assert_eq!(
            r.writes,
            std::vec![
                (0x200, 0x1100),
                (0x202, 0x2200),
                (0x204, 0x3300),
                (0x206, 0x4400)
            ]
        );
    }
}
