//! MOVEA.w / MOVEA.l (`00ss AAA0 01SS SSSs`): copy an operand into an address register, flags
//! untouched, a word is sign-extended. Timing: docs/specs/instr/MOVEA.md (Yacht.txt lines 529-559):
//! source phase then one prefetch, like MOVE to a data register.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{ea_read_plan, run_ea_hook, Ea, Size};
use crate::engine::plan::{Plan, Tok};

/// Identifier of MOVEA.
pub const ID: InsnId = InsnId(3);

const HK_SRC_DN: u8 = 0x20;
const HK_SRC_AN: u8 = 0x21;
const HK_DST_AN: u8 = 0x33;
const SRC_FORMS: usize = 12;

const fn src_ea(s: usize) -> Ea {
    match s {
        0 => Ea::DataReg(0),
        1 => Ea::AddrReg(0),
        2 => Ea::Ind(0),
        3 => Ea::PostInc(0),
        4 => Ea::PreDec(0),
        5 => Ea::Disp(0),
        6 => Ea::Index(0),
        7 => Ea::AbsW,
        8 => Ea::AbsL,
        9 => Ea::PcDisp,
        10 => Ea::PcIndex,
        _ => Ea::Imm,
    }
}

const fn build_rules() -> [Rule; 2 * SRC_FORMS] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; 2 * SRC_FORMS];
    let size_bits = [0x3000u16, 0x2000]; // w, l
    let mut k = 0;
    let mut sz = 0;
    while sz < 2 {
        let mut s = 0;
        while s < SRC_FORMS {
            let (smask, sval) = if s < 7 {
                (0x0038u16, (s as u16) << 3)
            } else {
                (0x003Fu16, (7u16 << 3) | (s as u16 - 7))
            };
            out[k] = Rule {
                mask: 0xF1C0 | smask,
                value: size_bits[sz] | 0x0040 | sval,
                id: ID,
                variant: sz as u8,
            };
            k += 1;
            s += 1;
        }
        sz += 1;
    }
    out
}

static RULES_ARR: [Rule; 2 * SRC_FORMS] = build_rules();

/// Decoding rules of MOVEA.
pub static RULES: &[Rule] = &RULES_ARR;

#[derive(Clone, Copy)]
struct Entry {
    toks: [Tok; 8],
    n: usize,
}

const fn build_plans() -> [Entry; 2 * SRC_FORMS] {
    let mut out = [Entry {
        toks: [Tok::N(0); 8],
        n: 0,
    }; 2 * SRC_FORMS];
    let mut sz = 0;
    while sz < 2 {
        let size = if sz == 0 { Size::Word } else { Size::Long };
        let mut s = 0;
        while s < SRC_FORMS {
            let mut e = Entry {
                toks: [Tok::N(0); 8],
                n: 0,
            };
            if s < 2 {
                e.toks[0] = Tok::H(if s == 0 { HK_SRC_DN } else { HK_SRC_AN });
                e.n = 1;
            } else {
                let p = ea_read_plan(src_ea(s), size);
                let mut i = 0;
                while i < p.len() {
                    e.toks[e.n] = p[i];
                    e.n += 1;
                    i += 1;
                }
            }
            e.toks[e.n] = Tok::H(HK_DST_AN);
            e.toks[e.n + 1] = Tok::Pf;
            e.n += 2;
            out[sz * SRC_FORMS + s] = e;
            s += 1;
        }
        sz += 1;
    }
    out
}

static PLANS: [Entry; 2 * SRC_FORMS] = build_plans();

fn src_form(ird: u16) -> usize {
    let mode = ((ird >> 3) & 7) as usize;
    if mode < 7 {
        mode
    } else {
        7 + (ird & 7) as usize
    }
}

fn size_of(ird: u16) -> Size {
    if (ird >> 12) & 3 == 3 {
        Size::Word
    } else {
        Size::Long
    }
}

/// Plan of the instruction being executed (opcode in the `ird` latch).
pub fn plan(variant: u8, cpu: &Cpu) -> Plan {
    let e = &PLANS[variant as usize * SRC_FORMS + src_form(cpu.lat.ird)];
    &e.toks[..e.n]
}

/// Semantics of the hooks of MOVEA.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of(ird);
    let src_reg = (ird & 7) as u8;
    match k {
        HK_SRC_DN => cpu.lat.val = cpu.regs.d[src_reg as usize],
        HK_SRC_AN => cpu.lat.val = cpu.regs.an(src_reg),
        HK_DST_AN => {
            let v = match size {
                Size::Word => cpu.lat.val as u16 as i16 as i32 as u32,
                _ => cpu.lat.val,
            };
            cpu.regs.set_an(((ird >> 9) & 7) as u8, v);
        }
        _ => {
            run_ea_hook(cpu, k, src_reg, size);
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

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0x8001
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, st: CpuState) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, 0x0100];
        cpu.load_state(&st);
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    #[test]
    fn e06_03_decode_legal() {
        // 8 destination registers x 61 sources x 2 sizes
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 * 61 * 2);
        assert_eq!(decode(0x3040).variant, 0); // MOVEA.w D0,A0
        assert_eq!(decode(0x2240).variant, 1); // MOVEA.l D0,A1
    }

    #[test]
    fn e06_03_decode_illegal() {
        // byte size (0x1040 is not MOVEA), source 7/5, and MOVE to Dn
        for op in [0x1040u16, 0x307D, 0x3000, 0x30C0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_03_plan_basic() {
        let clocks = |op: u16| {
            let mut cpu = Cpu::new();
            cpu.lat.ird = op;
            plan_clocks(plan(decode(op).variant, &cpu))
        };
        assert_eq!(clocks(0x3040), 4); // MOVEA.w D0,A0
        assert_eq!(clocks(0x3050), 8); // MOVEA.w (A0),A0
        assert_eq!(clocks(0x2050), 12); // MOVEA.l (A0),A0
        assert_eq!(clocks(0x207C), 12); // MOVEA.l #imm,A0
    }

    #[test]
    fn e06_03_flags_sample() {
        // flags never change; a word is sign-extended
        for (op, d0, want) in [
            (0x3040u16, 0x8000u32, 0xFFFF_8000u32),
            (0x3040, 0x1234, 0x1234),
            (0x2040, 0x8000, 0x8000),
        ] {
            let st = CpuState {
                d: [d0, 0, 0, 0, 0, 0, 0, 0],
                sr: 0x2700 | X | N | Z | V | C,
                pc: 0x100,
                ..CpuState::default()
            };
            let (cpu, n) = run(op, st);
            assert_eq!(n, 4);
            assert_eq!(cpu.regs.a[0], want);
            assert_eq!(cpu.regs.sr, 0x2700 | X | N | Z | V | C);
        }
    }
}
