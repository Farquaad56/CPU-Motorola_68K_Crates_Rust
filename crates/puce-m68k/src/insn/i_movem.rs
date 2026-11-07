//! MOVEM.w / MOVEM.l (`0100 1d00 1sEE Eeee` + register mask word).
//!
//! `d` = 0: registers to memory (modes (An), -(An), d16(An), d8(An,Xn), abs.W, abs.L);
//! `d` = 1: memory to registers (the same plus (An)+, d16(PC), d8(PC,Xn)). Timing:
//! docs/specs/instr/MOVEM.md (Yacht.txt lines 655-699), `m` = number of registers moved.
//!
//! The plan depends on `m`, known when the instruction starts (the mask is the word in IRC), so all
//! plans (direction, size, form, m = 0..=16) are built at compile time. Hooks pick the next
//! register of the mask (`lat.aux`: bits 0-15 remaining mask, bits 16-19 current register 0-15, D0-D7
//! then A0-A7) and advance the address (`lat.ea`).
//!
//! * memory to registers: `np ... (Rx, store)*m, extra read, np`; a word is sign-extended to 32 bits,
//!   one more word is read after the last one (Yacht note), (An)+ ends with An = final address.
//! * registers to memory: `np ... (write)*m, np`; for -(An) the mask is reversed (bit 0 = A7), the
//!   registers go from A7 down to D0, a long is written low word first, and An ends with the final
//!   address.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{
    run_ea_hook, Size, HK_EA_ABSL_HI, HK_EA_ABSL_LO, HK_EA_ABSW, HK_EA_AN, HK_EA_DISP, HK_EA_INDEX,
    HK_EA_PCDISP, HK_EA_PCINDEX,
};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of MOVEM.
pub const ID: InsnId = InsnId(5);

const HK_MASK: u8 = 0x50; // aux = register mask (IRC), before the Pf that consumes it
const HK_NEXT: u8 = 0x51; // R->M: next register of the mask into val
const HK_NEXT_PRE: u8 = 0x52; // R->M, -(An): next register (reversed mask), ea -= size
const HK_NEXT_R: u8 = 0x53; // M->R: next register of the mask (index only)
const HK_STORE: u8 = 0x54; // M->R: register = val (word sign-extended), ea += size
const HK_ADV: u8 = 0x55; // R->M: ea += size
const HK_SHIFT16: u8 = 0x56;
const HK_FIN: u8 = 0x57; // An = ea (postincrement / predecrement)

const MAXT: usize = 128;
const MAXM: usize = 17;
/// Forms of the address operand. R->M: (An), -(An), d16, d8, absW, absL.
/// M->R: (An), (An)+, d16, d8, absW, absL, d16(PC), d8(PC,Xn).
const FORMS: usize = 8;
const N_RULES: usize = 2 * (6 + 8);

const fn build_rules() -> [Rule; N_RULES] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES];
    let mut k = 0;
    let mut dir = 0; // 0: R->M, 1: M->R
    while dir < 2 {
        let mut sz = 0; // 0 word, 1 long
        while sz < 2 {
            let base: u16 = if dir == 0 { 0x4880 } else { 0x4C80 } | if sz == 1 { 0x40 } else { 0 };
            let forms = if dir == 0 { 6 } else { 8 };
            let mut f = 0;
            while f < forms {
                // (mask, value) of the effective address field for form f
                let (m, v): (u16, u16) = if dir == 0 {
                    match f {
                        0 => (0x38, 2 << 3),
                        1 => (0x38, 4 << 3),
                        2 => (0x38, 5 << 3),
                        3 => (0x38, 6 << 3),
                        4 => (0x3F, 0x38),
                        _ => (0x3F, 0x39),
                    }
                } else {
                    match f {
                        0 => (0x38, 2 << 3),
                        1 => (0x38, 3 << 3),
                        2 => (0x38, 5 << 3),
                        3 => (0x38, 6 << 3),
                        4 => (0x3F, 0x38),
                        5 => (0x3F, 0x39),
                        6 => (0x3F, 0x3A),
                        _ => (0x3F, 0x3B),
                    }
                };
                out[k] = Rule {
                    mask: 0xFFC0 | m,
                    value: base | v,
                    id: ID,
                    variant: (dir * 2 + sz) as u8,
                };
                k += 1;
                f += 1;
            }
            sz += 1;
        }
        dir += 1;
    }
    out
}

static RULES_ARR: [Rule; N_RULES] = build_rules();

/// Decoding rules of MOVEM.
pub static RULES: &[Rule] = &RULES_ARR;

#[derive(Clone, Copy)]
struct Entry {
    toks: [Tok; MAXT],
    n: usize,
}

const fn push(mut e: Entry, t: Tok) -> Entry {
    e.toks[e.n] = t;
    e.n += 1;
    e
}

const fn build_plan(dir: usize, size: Size, form: usize, m: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let long = matches!(size, Size::Long);
    let predec = dir == 0 && form == 1;
    let postinc = dir == 1 && form == 1;
    // forms are numbered per direction; map to an addressing kind: 0 (An), 1 predec/postinc,
    // 2 d16, 3 d8, 4 absW, 5 absL, 6 d16(PC), 7 d8(PC)
    e = push(e, Tok::H(HK_MASK));
    e = push(e, Tok::Pf); // consumes the mask word
    match form {
        0 | 1 => e = push(e, Tok::H(HK_EA_AN)),
        2 => {
            e = push(e, Tok::H(HK_EA_DISP));
            e = push(e, Tok::Pf);
        }
        3 => {
            e = push(e, Tok::H(HK_EA_INDEX));
            e = push(e, Tok::N(2));
            e = push(e, Tok::Pf);
        }
        4 => {
            e = push(e, Tok::H(HK_EA_ABSW));
            e = push(e, Tok::Pf);
        }
        5 => {
            e = push(e, Tok::H(HK_EA_ABSL_HI));
            e = push(e, Tok::Pf);
            e = push(e, Tok::H(HK_EA_ABSL_LO));
            e = push(e, Tok::Pf);
        }
        6 => {
            e = push(e, Tok::H(HK_EA_PCDISP));
            e = push(e, Tok::Pf);
        }
        _ => {
            e = push(e, Tok::H(HK_EA_PCINDEX));
            e = push(e, Tok::N(2));
            e = push(e, Tok::Pf);
        }
    }
    let src = if dir == 1 && form >= 6 {
        AddrSrc::EaProg
    } else {
        AddrSrc::Ea
    };
    let mut i = 0;
    while i < m {
        if dir == 1 {
            e = push(e, Tok::H(HK_NEXT_R));
            e = push(e, if long { Tok::Rl(src) } else { Tok::Rw(src) });
            e = push(e, Tok::H(HK_STORE));
        } else if predec {
            e = push(e, Tok::H(HK_NEXT_PRE));
            if long {
                e = push(e, Tok::Ww(AddrSrc::EaPlus2));
                e = push(e, Tok::H(HK_SHIFT16));
                e = push(e, Tok::Ww(AddrSrc::Ea));
            } else {
                e = push(e, Tok::Ww(AddrSrc::Ea));
            }
        } else {
            e = push(e, Tok::H(HK_NEXT));
            e = push(
                e,
                if long {
                    Tok::Wl(AddrSrc::Ea)
                } else {
                    Tok::Ww(AddrSrc::Ea)
                },
            );
            e = push(e, Tok::H(HK_ADV));
        }
        i += 1;
    }
    if dir == 1 {
        e = push(e, Tok::Rw(src)); // the extra read after the last word
    }
    if predec || postinc {
        e = push(e, Tok::H(HK_FIN));
    }
    push(e, Tok::Pf)
}

const fn build_plans() -> [Entry; 4 * FORMS * MAXM] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; 4 * FORMS * MAXM];
    let mut v = 0;
    while v < 4 {
        let dir = v / 2;
        let size = if v % 2 == 1 { Size::Long } else { Size::Word };
        let forms = if dir == 0 { 6 } else { 8 };
        let mut f = 0;
        while f < forms {
            let mut m = 0;
            while m < MAXM {
                out[(v * FORMS + f) * MAXM + m] = build_plan(dir, size, f, m);
                m += 1;
            }
            f += 1;
        }
        v += 1;
    }
    out
}

static PLANS: [Entry; 4 * FORMS * MAXM] = build_plans();

fn form_of(ird: u16, dir: usize) -> usize {
    let mode = (ird >> 3) & 7;
    let reg = ird & 7;
    match (dir, mode, reg) {
        (_, 2, _) => 0,
        (0, 4, _) | (1, 3, _) => 1,
        (_, 5, _) => 2,
        (_, 6, _) => 3,
        (_, 7, 0) => 4,
        (_, 7, 1) => 5,
        (_, 7, 2) => 6,
        _ => 7,
    }
}

/// Plan of the instruction being executed: it depends on the number of bits of the mask (IRC).
pub fn plan(variant: u8, cpu: &Cpu) -> Plan {
    let dir = (variant / 2) as usize;
    let m = cpu.regs.irc.count_ones() as usize;
    let e = &PLANS[(variant as usize * FORMS + form_of(cpu.lat.ird, dir)) * MAXM + m];
    &e.toks[..e.n]
}

fn reg_get(cpu: &Cpu, i: u32) -> u32 {
    if i < 8 {
        cpu.regs.d[i as usize]
    } else {
        cpu.regs.an((i - 8) as u8)
    }
}

fn reg_set(cpu: &mut Cpu, i: u32, v: u32) {
    if i < 8 {
        cpu.regs.d[i as usize] = v;
    } else {
        cpu.regs.set_an((i - 8) as u8, v);
    }
}

/// Takes the lowest set bit of the remaining mask; returns its position.
fn take_bit(cpu: &mut Cpu) -> u32 {
    let mask = cpu.lat.aux & 0xFFFF;
    let b = mask.trailing_zeros();
    cpu.lat.aux = (cpu.lat.aux & !0xFFFF) | (mask & mask.wrapping_sub(1));
    b & 15
}

/// Semantics of the hooks of MOVEM.
pub fn hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let long = ird & 0x40 != 0;
    let step: u32 = if long { 4 } else { 2 };
    match k {
        HK_MASK => cpu.lat.aux = cpu.regs.irc as u32,
        HK_NEXT => {
            let i = take_bit(cpu);
            cpu.lat.val = reg_get(cpu, i);
        }
        HK_NEXT_PRE => {
            let j = take_bit(cpu);
            cpu.lat.val = reg_get(cpu, 15 - j);
            cpu.lat.ea = cpu.lat.ea.wrapping_sub(step);
        }
        HK_NEXT_R => {
            let i = take_bit(cpu);
            cpu.lat.aux = (cpu.lat.aux & 0xFFFF) | (i << 16);
        }
        HK_STORE => {
            let i = cpu.lat.aux >> 16;
            let v = if long {
                cpu.lat.val
            } else {
                cpu.lat.val as u16 as i16 as i32 as u32
            };
            reg_set(cpu, i, v);
            cpu.lat.ea = cpu.lat.ea.wrapping_add(step);
        }
        HK_ADV => cpu.lat.ea = cpu.lat.ea.wrapping_add(step),
        HK_SHIFT16 => cpu.lat.val >>= 16,
        HK_FIN => {
            let v = cpu.lat.ea;
            cpu.regs.set_an((ird & 7) as u8, v);
        }
        _ => {
            let size = if long { Size::Long } else { Size::Word };
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

    fn run(op: u16, mask: u16, st: CpuState, ram: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        let mut st = st;
        st.prefetch = [op, mask];
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
    fn e06_05a_decode_legal() {
        // R->M: 5 forms with a free register field (An 8 x modes (An), -(An), d16, d8) + 2 absolute
        let rm = 4 * 8 + 2;
        assert_eq!(
            TABLE.iter().filter(|d| d.id == ID && d.variant < 2).count(),
            2 * rm
        );
        assert_eq!(decode(0x4890).variant, 0); // MOVEM.w regs,(A0)
        assert_eq!(decode(0x48D0).variant, 1); // MOVEM.l regs,(A0)
        assert_eq!(decode(0x4C90).variant, 2);
        assert_eq!(decode(0x4CD0).variant, 3);
    }

    #[test]
    fn e06_05a_decode_illegal() {
        // mode 0 is EXT, mode 3 (R->M) and mode 4 (M->R) do not exist, 7/4 and PC modes (R->M)
        for op in [0x4880u16, 0x48C0, 0x4898, 0x4CA0, 0x48BA, 0x48BC, 0x4CBC] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_05a_plan_basic() {
        let clocks = |op: u16, mask: u16| {
            let mut cpu = Cpu::new();
            cpu.lat.ird = op;
            cpu.regs.irc = mask;
            plan_clocks(plan(decode(op).variant, &cpu))
        };
        assert_eq!(clocks(0x4890, 0x0003), 8 + 4 * 2); // .W R->M (An), m = 2
        assert_eq!(clocks(0x48D0, 0x0003), 8 + 8 * 2); // .L
        assert_eq!(clocks(0x4C90, 0x0003), 12 + 4 * 2); // .W M->R (An)
        assert_eq!(clocks(0x4CD0, 0x0003), 12 + 8 * 2);
        assert_eq!(clocks(0x48A8, 0x0007), 12 + 4 * 3); // .W R->M d16(An)
    }

    #[test]
    fn e06_05a_flags_sample() {
        // flags are never touched; D0/D1 stored in order, A0 untouched
        let mut r = ram();
        let st = CpuState {
            d: [0x1111_2222, 0x3333_4444, 0, 0, 0, 0, 0, 0],
            a: [0x200, 0, 0, 0, 0, 0, 0],
            sr: 0x271F,
            pc: 0x100,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x48D0, 0x0003, st, &mut r); // MOVEM.l D0-D1,(A0)
        assert_eq!(n, 24);
        assert_eq!(cpu.regs.sr, 0x271F);
        assert_eq!(
            r.writes,
            std::vec![
                (0x200, 0x1111),
                (0x202, 0x2222),
                (0x204, 0x3333),
                (0x206, 0x4444)
            ]
        );
    }

    #[test]
    fn e06_05a_predecrement_reversed() {
        let mut r = ram();
        let st = CpuState {
            d: [0xAAAA_0001, 0, 0, 0, 0, 0, 0, 0],
            a: [0, 0, 0, 0, 0, 0, 0],
            usp: 0x300,
            sr: 0x0000,
            pc: 0x100,
            ..CpuState::default()
        };
        // MOVEM.w D0/A7,-(A7): mask reversed, bit 0 = A7, bit 15 = D0
        let (cpu, n) = run(0x48A7, 0x8001, st, &mut r);
        assert_eq!(n, 8 + 4 * 2);
        // A7 (initial value 0x300) first, then D0
        assert_eq!(r.writes, std::vec![(0x2FE, 0x0300), (0x2FC, 0x0001)]);
        assert_eq!(cpu.regs.usp, 0x2FC);
    }

    #[test]
    fn e06_05b_decode_legal() {
        // M->R: modes (An), (An)+, d16, d8 (8 registers each) + absW, absL, d16(PC), d8(PC)
        let mr = 4 * 8 + 4;
        assert_eq!(
            TABLE
                .iter()
                .filter(|d| d.id == ID && d.variant >= 2)
                .count(),
            2 * mr
        );
        assert_eq!(decode(0x4C98).variant, 2); // (A0)+
        assert_eq!(decode(0x4CFA).variant, 3); // d16(PC), long
    }

    #[test]
    fn e06_05b_decode_illegal() {
        for op in [0x4C80u16, 0x4CA0, 0x4CBC, 0x4CC0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_05b_plan_basic() {
        let mut cpu = Cpu::new();
        cpu.lat.ird = 0x4C90;
        cpu.regs.irc = 0x0003;
        assert_eq!(plan_clocks(plan(2, &cpu)), 12 + 4 * 2);
        cpu.lat.ird = 0x4C98; // (An)+
        assert_eq!(plan_clocks(plan(2, &cpu)), 12 + 4 * 2);
    }

    #[test]
    fn e06_05b_flags_sample() {
        // word sign-extended into the whole register, SR untouched
        let mut r = ram();
        r.w[0x100] = 0x8000;
        r.w[0x101] = 0x7FFF;
        let st = CpuState {
            a: [0x200, 0, 0, 0, 0, 0, 0],
            sr: 0x2715,
            pc: 0x20,
            ..CpuState::default()
        };
        let (cpu, n) = run(0x4C90, 0x0003, st, &mut r); // MOVEM.w (A0),D0-D1
        assert_eq!(n, 20);
        assert_eq!(cpu.regs.d[0], 0xFFFF_8000);
        assert_eq!(cpu.regs.d[1], 0x0000_7FFF);
        assert_eq!(cpu.regs.sr, 0x2715);
    }
}
