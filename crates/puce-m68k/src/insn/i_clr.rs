//! CLR.b / CLR.w / CLR.l (`0100 0010 ss ea`): clears a data-alterable operand. The 68000 reads the
//! operand first (discarded), then writes zero: N = 0, Z = 1, V = C = 0, X untouched.
//! Timing: docs/specs/instr/CLR.md. Variant = size (0 b, 1 w, 2 l).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};

use crate::ea::{ea_read_plan, run_ea_hook, Ea, Size};
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::regs::{C, N, V, Z};

/// Identifier of CLR.
pub const ID: InsnId = InsnId(14);

const HK_CLR_DN: u8 = 0x70;
const HK_ZERO: u8 = 0x71;
const FORMS: usize = 8;

const fn build_rules() -> [Rule; 3 * FORMS] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; 3 * FORMS];
    let mut s = 0;
    let mut k = 0;
    while s < 3 {
        let base: u16 = 0x4200 | ((s as u16) << 6);
        let mut f = 0;
        while f < FORMS {
            let (mask, value): (u16, u16) = match f {
                0 => (0xFFF8, base),
                1 => (0xFFF8, base | (2 << 3)),
                2 => (0xFFF8, base | (3 << 3)),
                3 => (0xFFF8, base | (4 << 3)),
                4 => (0xFFF8, base | (5 << 3)),
                5 => (0xFFF8, base | (6 << 3)),
                6 => (0xFFFF, base | 0x38),
                _ => (0xFFFF, base | 0x39),
            };
            out[k] = Rule {
                mask,
                value,
                id: ID,
                variant: s as u8,
            };
            k += 1;
            f += 1;
        }
        s += 1;
    }
    out
}

static RULES_ARR: [Rule; 3 * FORMS] = build_rules();

/// Decoding rules.
pub static RULES: &[Rule] = &RULES_ARR;

const fn form_ea(f: usize) -> Ea {
    match f {
        0 => Ea::DataReg(0),
        1 => Ea::Ind(0),
        2 => Ea::PostInc(0),
        3 => Ea::PreDec(0),
        4 => Ea::Disp(0),
        5 => Ea::Index(0),
        6 => Ea::AbsW,
        _ => Ea::AbsL,
    }
}

#[derive(Clone, Copy)]
struct Entry {
    toks: [Tok; 16],
    n: usize,
}

const fn push(mut e: Entry, t: Tok) -> Entry {
    e.toks[e.n] = t;
    e.n += 1;
    e
}

const fn size_of_index(s: usize) -> Size {
    match s {
        0 => Size::Byte,
        1 => Size::Word,
        _ => Size::Long,
    }
}

const fn build_plan(s: usize, f: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); 16],
        n: 0,
    };
    if f == 0 {
        e = push(e, Tok::H(HK_CLR_DN));
        e = push(e, Tok::Pf);
        if s == 2 {
            e = push(e, Tok::N(2));
        }
        return e;
    }
    let p = ea_read_plan(form_ea(f), size_of_index(s));
    let mut i = 0;
    while i < p.len() {
        e = push(e, p[i]);
        i += 1;
    }
    e = push(e, Tok::H(HK_ZERO));
    e = push(e, Tok::Pf);
    if s == 2 {
        e = push(e, Tok::Ww(AddrSrc::EaPlus2));
        return push(e, Tok::Ww(AddrSrc::Ea));
    }
    push(
        e,
        match s {
            0 => Tok::Wb(AddrSrc::Ea),
            1 => Tok::Ww(AddrSrc::Ea),
            _ => Tok::Ww(AddrSrc::EaPlus2),
        },
    )
}

const fn build_plans() -> [Entry; 3 * FORMS] {
    let mut out = [Entry {
        toks: [Tok::N(0); 16],
        n: 0,
    }; 3 * FORMS];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < FORMS {
            out[s * FORMS + f] = build_plan(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

static PLANS: [Entry; 3 * FORMS] = build_plans();

fn form_of(ird: u16) -> usize {
    match (ird >> 3) & 7 {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        _ => 6 + (ird & 7) as usize,
    }
}

/// Plan of the instruction being executed.
pub fn plan(variant: u8, cpu: &Cpu) -> Plan {
    let e = &PLANS[variant as usize * FORMS + form_of(cpu.lat.ird)];
    &e.toks[..e.n]
}

fn set_flags(cpu: &mut Cpu) {
    cpu.regs.sr = (cpu.regs.sr & !(N | V | C)) | Z;
}

/// Semantics of the hooks of CLR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    match k {
        HK_CLR_DN => {
            let r = (ird & 7) as usize;
            let keep = match (ird >> 6) & 3 {
                0 => 0xFFFF_FF00,
                1 => 0xFFFF_0000,
                _ => 0,
            };
            cpu.regs.d[r] &= keep;
            set_flags(cpu);
        }
        HK_ZERO => {
            cpu.lat.val = 0;
            set_flags(cpu);
        }
        _ => {
            let size = size_of_index(((ird >> 6) & 3) as usize);
            run_ea_hook(cpu, k, (ird & 7) as u8, size);
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
    fn e06_14_decode_legal() {
        assert_eq!(
            TABLE.iter().filter(|d| d.id == ID).count(),
            3 * (8 + 5 * 8 + 2)
        );
        assert_eq!(decode(0x4210).variant, 0);
        assert_eq!(decode(0x4250).variant, 1);
        assert_eq!(decode(0x4299).variant, 2);
    }

    #[test]
    fn e06_14_decode_illegal() {
        for op in [0x4208u16, 0x423A, 0x423C, 0x42C0, 0x4248] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_14_plan_basic() {
        let c = |op: u16| {
            let mut cpu = Cpu::new();
            cpu.lat.ird = op;
            plan_clocks(plan(((op >> 6) & 3) as u8, &cpu))
        };
        assert_eq!(c(0x4200), 4); // CLR.B D0
        assert_eq!(c(0x4280), 6); // CLR.L D0
        assert_eq!(c(0x4210), 12); // CLR.B (A0)
        assert_eq!(c(0x4290), 20); // CLR.L (A0)
        assert_eq!(c(0x4268), 8 + 8); // CLR.W d16(A0) = 8 + 8 (EA) ... see table
    }

    #[test]
    fn e06_14_flags_sample() {
        let mut r = ram();
        let st = CpuState {
            d: [0x1234_5678, 0, 0, 0, 0, 0, 0, 0],
            a: [0x200, 0, 0, 0, 0, 0, 0],
            sr: X | N | V | C,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4240, 0, st, &mut r); // CLR.W D0
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr), (4, 0x1234_0000, X | Z));
        let (cpu, n) = run(0x4290, 0, st, &mut r); // CLR.L (A0)
        assert_eq!(n, 20);
        assert_eq!(cpu.regs.sr, X | Z);
        assert_eq!(r.writes, std::vec![(0x202, 0), (0x200, 0)]);
    }
}
