//! MOVE.b / MOVE.w / MOVE.l (`00ss DDDd ddSS SSSs`): copy an operand, N and Z set, V and C cleared.
//!
//! Timing: docs/specs/instr/MOVE.md (Yacht.txt lines 340-528). The plan of every legal
//! (size, source form, destination form) is built at compile time from the shared EA plans:
//! source phase, flags, destination phase, last prefetch. Destination `-(An)` is the exception: its
//! prefetch comes BEFORE the write, and a long is written low word first (`np nw nW`).
//! MOVE to An is MOVEA (a separate instruction) and is not captured here.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    ea_read_plan, ea_write_plan, run_ea_hook, Ea, Size, HK_AN_PREDEC, HK_EA_ABSL_HI, HK_EA_ABSL_LO,
};
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::regs::{C, N, V, Z};

/// Identifier of MOVE.
pub const ID: InsnId = InsnId(2);

const HK_SRC_DN: u8 = 0x20;
const HK_SRC_AN: u8 = 0x21;
const HK_FLAGS: u8 = 0x30;
const HK_DST_DN: u8 = 0x31;
const HK_SHIFT16: u8 = 0x32;
/// Flag added to an EA hook id: the register is the destination one (bits 11-9 of the opcode).
const DST: u8 = 0x40;

/// Source forms: Dn, An, (An), (An)+, -(An), d16(An), d8(An,Xn), abs.W, abs.L, d16(PC), d8(PC,Xn), #imm.
const SRC_FORMS: usize = 12;
/// Destination forms: Dn, (An), (An)+, -(An), d16(An), d8(An,Xn), abs.W, abs.L.
const DST_FORMS: usize = 8;
const N_RULES: usize = 3 * DST_FORMS * SRC_FORMS - DST_FORMS; // no byte move from An

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

const fn dst_ea(d: usize) -> Ea {
    match d {
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

const fn size_of_index(i: usize) -> Size {
    match i {
        0 => Size::Byte,
        1 => Size::Word,
        _ => Size::Long,
    }
}

/// Decoding rules: one per (size, destination form, source form).
const fn build_rules() -> [Rule; N_RULES] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES];
    let size_bits = [0x1000u16, 0x3000, 0x2000]; // b, w, l
    let dst_modes = [0u16, 2, 3, 4, 5, 6];
    let mut k = 0;
    let mut sz = 0;
    while sz < 3 {
        let mut d = 0;
        while d < DST_FORMS {
            let (dmask, dval) = if d < 6 {
                (0x01C0u16, dst_modes[d] << 6)
            } else {
                (0x0FC0u16, (((d - 6) as u16) << 9) | (7 << 6))
            };
            let mut s = 0;
            while s < SRC_FORMS {
                if !(sz == 0 && s == 1) {
                    let (smask, sval) = if s < 7 {
                        (0x0038u16, (s as u16) << 3)
                    } else {
                        (0x003Fu16, (7u16 << 3) | (s as u16 - 7))
                    };
                    out[k] = Rule {
                        mask: 0xF000 | dmask | smask,
                        value: size_bits[sz] | dval | sval,
                        id: ID,
                        variant: sz as u8,
                    };
                    k += 1;
                }
                s += 1;
            }
            d += 1;
        }
        sz += 1;
    }
    out
}

static RULES_ARR: [Rule; N_RULES] = build_rules();

/// Decoding rules of MOVE.
pub static RULES: &[Rule] = &RULES_ARR;

/// A plan under construction (at most 16 tokens).
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

/// Appends a plan; `dst` marks the hooks as destination-side.
const fn push_all(mut e: Entry, p: Plan, dst: bool) -> Entry {
    let mut i = 0;
    while i < p.len() {
        let t = match p[i] {
            Tok::H(k) if dst => Tok::H(k | DST),
            t => t,
        };
        e = push(e, t);
        i += 1;
    }
    e
}

const fn build_plan(size: Size, s: usize, d: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); 16],
        n: 0,
    };
    // source phase
    e = match s {
        0 => push(e, Tok::H(HK_SRC_DN)),
        1 => push(e, Tok::H(HK_SRC_AN)),
        _ => push_all(e, ea_read_plan(src_ea(s), size), false),
    };
    e = push(e, Tok::H(HK_FLAGS));
    // destination phase
    match d {
        0 => e = push(e, Tok::H(HK_DST_DN)),
        3 => {
            // -(An): prefetch first, then the write (a long: low word first)
            e = push(e, Tok::H(HK_AN_PREDEC | DST));
            e = push(e, Tok::Pf);
            e = match size {
                Size::Byte => push(e, Tok::Wb(AddrSrc::Ea)),
                Size::Word => push(e, Tok::Ww(AddrSrc::Ea)),
                Size::Long => {
                    e = push(e, Tok::Ww(AddrSrc::EaPlus2));
                    e = push(e, Tok::H(HK_SHIFT16));
                    push(e, Tok::Ww(AddrSrc::Ea))
                }
            };
            return e;
        }
        7 if s >= 2 && s <= 10 => {
            // (xxx).L destination after a memory source: only one prefetch before the write,
            // two after it (Yacht.txt lines 478-488: `np nw np np`).
            e = push(e, Tok::H(HK_EA_ABSL_HI | DST));
            e = push(e, Tok::Pf);
            e = push(e, Tok::H(HK_EA_ABSL_LO | DST));
            e = match size {
                Size::Byte => push(e, Tok::Wb(AddrSrc::Ea)),
                Size::Word => push(e, Tok::Ww(AddrSrc::Ea)),
                Size::Long => push(e, Tok::Wl(AddrSrc::Ea)),
            };
            e = push(e, Tok::Pf);
        }
        _ => e = push_all(e, ea_write_plan(dst_ea(d), size), true),
    }
    push(e, Tok::Pf)
}

const fn build_plans() -> [Entry; 3 * SRC_FORMS * DST_FORMS] {
    let empty = Entry {
        toks: [Tok::N(0); 16],
        n: 0,
    };
    let mut out = [empty; 3 * SRC_FORMS * DST_FORMS];
    let mut sz = 0;
    while sz < 3 {
        let mut s = 0;
        while s < SRC_FORMS {
            let mut d = 0;
            while d < DST_FORMS {
                if !(sz == 0 && s == 1) {
                    out[(sz * SRC_FORMS + s) * DST_FORMS + d] = build_plan(size_of_index(sz), s, d);
                }
                d += 1;
            }
            s += 1;
        }
        sz += 1;
    }
    out
}

static PLANS: [Entry; 3 * SRC_FORMS * DST_FORMS] = build_plans();

fn src_form(ird: u16) -> usize {
    let mode = ((ird >> 3) & 7) as usize;
    if mode < 7 {
        mode
    } else {
        7 + (ird & 7) as usize
    }
}

fn dst_form(ird: u16) -> usize {
    match (ird >> 6) & 7 {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        _ => 6 + ((ird >> 9) & 7) as usize,
    }
}

fn size_of(ird: u16) -> Size {
    match (ird >> 12) & 3 {
        1 => Size::Byte,
        3 => Size::Word,
        _ => Size::Long,
    }
}

/// Plan of the instruction being executed (the opcode is in the `ird` latch).
pub fn plan(variant: u8, cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let e = &PLANS[(variant as usize * SRC_FORMS + src_form(ird)) * DST_FORMS + dst_form(ird)];
    &e.toks[..e.n]
}

fn mask(size: Size, v: u32) -> u32 {
    match size {
        Size::Byte => v & 0xFF,
        Size::Word => v & 0xFFFF,
        Size::Long => v,
    }
}

/// Semantics of the hooks of MOVE.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of(ird);
    let src_reg = (ird & 7) as u8;
    let dst_reg = ((ird >> 9) & 7) as u8;
    match k {
        HK_SRC_DN => cpu.lat.val = mask(size, cpu.regs.d[src_reg as usize]),
        HK_SRC_AN => cpu.lat.val = mask(size, cpu.regs.an(src_reg)),
        HK_FLAGS => {
            let v = mask(size, cpu.lat.val);
            let msb = match size {
                Size::Byte => 0x80,
                Size::Word => 0x8000,
                Size::Long => 0x8000_0000,
            };
            let mut sr = cpu.regs.sr & !(N | Z | V | C);
            if v == 0 {
                sr |= Z;
            }
            if v & msb != 0 {
                sr |= N;
            }
            cpu.regs.sr = sr;
        }
        HK_DST_DN => {
            let old = cpu.regs.d[dst_reg as usize];
            let v = cpu.lat.val;
            cpu.regs.d[dst_reg as usize] = match size {
                Size::Byte => (old & !0xFF) | (v & 0xFF),
                Size::Word => (old & !0xFFFF) | (v & 0xFFFF),
                Size::Long => v,
            };
        }
        HK_SHIFT16 => cpu.lat.val >>= 16,
        _ => {
            let (reg, id) = if k & DST != 0 {
                (dst_reg, k & !DST)
            } else {
                (src_reg, k)
            };
            run_ea_hook(cpu, id, reg, size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::pins::Pins;
    use crate::regs::X;
    use crate::state::CpuState;

    /// Small RAM that records the writes (address, data, uds, lds).
    struct Ram {
        words: [u16; 0x400],
        writes: std::vec::Vec<(u32, u16, bool, bool)>,
    }
    impl Ram {
        fn new() -> Self {
            Ram {
                words: [0; 0x400],
                writes: std::vec::Vec::new(),
            }
        }
    }
    impl Bus for Ram {
        fn read(&mut self, a: u32, _fc: u8, _u: bool, _l: bool) -> u16 {
            self.words[(a as usize >> 1) & 0x3FF]
        }
        fn write(&mut self, a: u32, _fc: u8, u: bool, l: bool, d: u16) {
            self.writes.push((a, d, u, l));
            self.words[(a as usize >> 1) & 0x3FF] = d;
        }
        fn on_tick(&mut self, _p: Pins) {}
    }

    fn clocks_of(op: u16) -> usize {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        let d = decode(op);
        assert_eq!(d.id, ID, "{op:#06x} not decoded as MOVE");
        plan_clocks(plan(d.variant, &cpu)) as usize
    }

    fn run(op: u16, st: CpuState, ram: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, 0x0100];
        cpu.load_state(&st);
        let n = cpu.step(ram);
        (cpu, n)
    }

    #[test]
    fn e06_02a_decode_legal() {
        // 50 destinations x (61 + 61 + 53) sources (word, long, byte without An)
        assert_eq!(
            TABLE.iter().filter(|d| d.id == ID).count(),
            50 * (61 + 61 + 53)
        );
        assert_eq!(decode(0x3200).variant, 1); // MOVE.w D0,D1
        assert_eq!(decode(0x1200).variant, 0); // MOVE.b D0,D1
        assert_eq!(decode(0x2200).variant, 2); // MOVE.l D0,D1
    }

    #[test]
    fn e06_02a_decode_illegal() {
        // MOVEA (destination An), byte move from An, source mode 7/5, destination mode 7/2
        for op in [
            0x3248u16,
            0x2040,
            0x1008,
            0x303D,
            0x35C0 + 0x0200,
            0x0200,
            0x4200,
        ] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_02a_plan_basic() {
        assert_eq!(clocks_of(0x3200), 4); // MOVE.w D0,D1: Yacht line 346
        assert_eq!(clocks_of(0x3210), 8); // MOVE.w (A0),D1
        assert_eq!(clocks_of(0x1280), 8); // MOVE.b D0,(A1)
        assert_eq!(clocks_of(0x3280), 8); // MOVE.w D0,(A1)... (An) destination: nw np
        assert_eq!(clocks_of(0x2280), 12); // MOVE.l D0,(A1): nW nw np
        assert_eq!(clocks_of(0x2300), 12); // MOVE.l D0,-(A1): np nw nW
    }

    #[test]
    fn e06_02a_flags_sample() {
        // MOVE.w D0,D1 with D0 = 0 -> Z; 0x8000 -> N; 0x1234 -> none; X kept, V and C cleared
        let cases = [(0u32, Z), (0x8000, N), (0x1234, 0)];
        for (v, want) in cases {
            let mut ram = Ram::new();
            let st = CpuState {
                d: [v, 0xFFFF_FFFF, 0, 0, 0, 0, 0, 0],
                sr: 0x2700 | X | V | C,
                pc: 0x100,
                ..CpuState::default()
            };
            let (cpu, n) = run(0x3200, st, &mut ram);
            assert_eq!(n, 4);
            assert_eq!(cpu.regs.sr & 0x1F, X | want, "value {v:#x}");
            assert_eq!(cpu.regs.d[1], 0xFFFF_0000 | v, "upper half of D1 kept");
        }
    }

    #[test]
    fn e06_02b_long_predec_writes_low_word_first() {
        let mut ram = Ram::new();
        let mut st = CpuState {
            sr: 0x2700,
            pc: 0x100,
            ..CpuState::default()
        };
        st.d[0] = 0x1122_3344;
        st.a[1] = 0x200;
        let (cpu, n) = run(0x2300, st, &mut ram); // MOVE.l D0,-(A1)
        assert_eq!(n, 12);
        assert_eq!(cpu.regs.a[1], 0x1FC);
        let w: std::vec::Vec<_> = ram.writes.iter().map(|x| (x.0, x.1)).collect();
        assert_eq!(w, std::vec![(0x1FE, 0x3344), (0x1FC, 0x1122)]);
    }

    #[test]
    fn e06_02b_memory_to_memory() {
        // MOVE.w (A0)+,(d16,A1) with d16 = 0x10 in the extension word
        let mut ram = Ram::new();
        ram.words[0x80] = 0xBEEF; // at 0x100
        let st = CpuState {
            sr: 0x2700,
            a: [0x100, 0x300, 0, 0, 0, 0, 0],
            pc: 0x400,
            ..CpuState::default()
        };
        let mut cpu = Cpu::new();
        let mut st2 = st;
        st2.prefetch = [0x3358, 0x0010]; // MOVE.w (A0)+,$10(A1)
        cpu.load_state(&st2);
        let n = cpu.step(&mut ram);
        assert_eq!(n, 16); // Yacht: <ea>,(d16,An) .W (An)+ = 16(3/1)
        assert_eq!(cpu.regs.a[0], 0x102);
        assert_eq!(ram.writes, std::vec![(0x310, 0xBEEF, true, true)]);
    }

    #[test]
    fn e06_02b_byte_a7_step() {
        // MOVE.b D0,-(A7) decrements A7 by 2
        let mut ram = Ram::new();
        let mut st = CpuState {
            sr: 0x0000,
            pc: 0x100,
            usp: 0x300,
            ..CpuState::default()
        };
        st.d[0] = 0x55;
        let (cpu, _) = run(0x1F00, st, &mut ram);
        assert_eq!(cpu.regs.usp, 0x2FE);
    }

    #[test]
    fn e06_02b_decode_legal() {
        let count = |v: u8| {
            TABLE
                .iter()
                .filter(|d| d.id == ID && d.variant == v)
                .count()
        };
        assert_eq!(count(0), 50 * 53); // byte: no source An
        assert_eq!(count(1), 50 * 61);
        assert_eq!(count(2), 50 * 61);
        // every source form decodes, e.g. MOVE.w <src>,D1
        for src in [
            0x00u16, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x39, 0x3A, 0x3B, 0x3C,
        ] {
            assert_eq!(decode(0x3200 | src).id, ID, "{src:#x}");
        }
    }

    #[test]
    fn e06_02b_decode_illegal() {
        // byte move from An0..An7, destination d16(PC) / d8(PC,Xn) / #imm, source 7/5..7/7
        for r in 0..8u16 {
            assert_ne!(decode(0x1008 | r).id, ID);
        }
        for dst in [0x35C0u16, 0x37C0, 0x39C0] {
            assert_ne!(decode(dst).id, ID, "{dst:#x}");
        }
        for src in [0x3Du16, 0x3E, 0x3F] {
            assert_ne!(decode(0x3200 | src).id, ID, "{src:#x}");
        }
    }

    #[test]
    fn e06_02b_plan_basic() {
        assert_eq!(clocks_of(0x23F9), 36); // MOVE.l (xxx).L,(xxx).L: Yacht 36(7/2)
        assert_eq!(clocks_of(0x33FC), 20); // MOVE.w #imm,(xxx).L: 20(4/1)
        assert_eq!(clocks_of(0x3030), 14); // MOVE.w (d8,A0,Xn),D0: 14(3/0)
        assert_eq!(clocks_of(0x2F3C), 20); // MOVE.l #imm,-(A7): 20(3/2)
    }

    #[test]
    fn e06_02b_flags_sample() {
        // (opcode, D0, expected N/Z): MOVE.b D0,D1; MOVE.l D0,D1
        let cases = [
            (0x1200u16, 0x80u32, N),
            (0x2200, 0, Z),
            (0x2200, 0x7FFF_FFFF, 0),
        ];
        for (op, v, want) in cases {
            let mut ram = Ram::new();
            let st = CpuState {
                d: [v, 0, 0, 0, 0, 0, 0, 0],
                sr: 0x2700 | V | C,
                pc: 0x100,
                ..CpuState::default()
            };
            let (cpu, n) = run(op, st, &mut ram);
            assert_eq!(n, 4);
            assert_eq!(cpu.regs.sr & 0x1F, want, "{op:#06x} {v:#x}");
        }
    }
}
