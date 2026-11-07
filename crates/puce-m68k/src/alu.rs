//! Pure integer ALU and the shared plans of the two-operand instructions
//! (`ADD`, `SUB`, `AND`, `OR`, `EOR`, `CMP` with `<ea>,Dn` / `Dn,<ea>` operands).
//!
//! Timing source: docs/specs/instr/ADD.md ("ADD, SUB" table). Opcode layout: `line | Dn << 9 |
//! opmode << 6 | ea` with opmode 0..2 = byte/word/long `<ea>,Dn` and 4..6 = `Dn,<ea>`.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::ea::{ea_read_plan, Ea, Size};
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::regs::{C, N, V, X, Z};

/// Operation of the ALU.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alu {
    /// `dst + src`.
    Add,
    /// `dst - src`.
    Sub,
    /// `dst - src`, flags only (X untouched).
    Cmp,
    /// `dst & src`.
    And,
    /// `dst | src`.
    Or,
    /// `dst ^ src`.
    Eor,
}

/// Mask of the bits of an operand of the given size.
pub const fn size_mask(size: Size) -> u32 {
    match size {
        Size::Byte => 0xFF,
        Size::Word => 0xFFFF,
        Size::Long => 0xFFFF_FFFF,
    }
}

/// Sign bit of an operand of the given size.
pub const fn size_msb(size: Size) -> u32 {
    match size {
        Size::Byte => 0x80,
        Size::Word => 0x8000,
        Size::Long => 0x8000_0000,
    }
}

/// `dst + src + carry_in`: result (masked) and the flags `(x/c, v)`.
pub fn add_flags(size: Size, src: u32, dst: u32, cin: u32) -> (u32, bool, bool) {
    let m = size_mask(size);
    let (s, d) = (src & m, dst & m);
    let wide = s as u64 + d as u64 + cin as u64;
    let r = (wide as u32) & m;
    let c = wide > m as u64;
    let v = (s ^ r) & (d ^ r) & size_msb(size) != 0;
    (r, c, v)
}

/// `dst - src - borrow_in`: result (masked) and the flags `(x/c, v)`.
pub fn sub_flags(size: Size, src: u32, dst: u32, bin: u32) -> (u32, bool, bool) {
    let m = size_mask(size);
    let (s, d) = (src & m, dst & m);
    let r = d.wrapping_sub(s).wrapping_sub(bin) & m;
    let c = (s as u64 + bin as u64) > d as u64;
    let v = (d ^ s) & (d ^ r) & size_msb(size) != 0;
    (r, c, v)
}

/// Builds the condition code bits N, Z (from `r`) plus the given V, C.
pub fn nz_flags(size: Size, r: u32) -> u16 {
    let mut f = 0;
    if r & size_mask(size) == 0 {
        f |= Z;
    }
    if r & size_msb(size) != 0 {
        f |= N;
    }
    f
}

/// Executes `op`: returns the result (masked to `size`) and the new status register.
pub fn exec(op: Alu, size: Size, src: u32, dst: u32, sr: u16) -> (u32, u16) {
    let keep = sr & !(X | N | Z | V | C);
    match op {
        Alu::Add | Alu::Sub | Alu::Cmp => {
            let (r, c, v) = if op == Alu::Add {
                add_flags(size, src, dst, 0)
            } else {
                sub_flags(size, src, dst, 0)
            };
            let mut f = nz_flags(size, r);
            if v {
                f |= V;
            }
            if c {
                f |= C;
            }
            if op != Alu::Cmp && c {
                f |= X;
            }
            let x = if op == Alu::Cmp { sr & X } else { 0 };
            (r, keep | f | x)
        }
        _ => {
            let m = size_mask(size);
            let r = match op {
                Alu::And => src & dst,
                Alu::Or => src | dst,
                _ => src ^ dst,
            } & m;
            (r, keep | (sr & X) | nz_flags(size, r))
        }
    }
}

/// Hook of the two-operand plans: computes the operation (after the operand read, if any).
pub const HK_BINOP: u8 = 0x80;

/// Size of an opcode of the `ADD` family (bits 7-6: 0 byte, 1 word, 2 long).
pub fn size_of_ird(ird: u16) -> Size {
    match (ird >> 6) & 3 {
        0 => Size::Byte,
        1 => Size::Word,
        _ => Size::Long,
    }
}

/// Writes `v` (masked) into the low part of a data register.
pub fn set_dn(cpu: &mut Cpu, n: usize, size: Size, v: u32) {
    let m = size_mask(size);
    cpu.regs.d[n] = (cpu.regs.d[n] & !m) | (v & m);
}

/// Semantics of [`HK_BINOP`] for the operation `op`.
pub fn binop_exec(op: Alu, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of_ird(ird);
    let dn = ((ird >> 9) & 7) as usize;
    let reg = (ird & 7) as u8;
    if ird & 0x100 == 0 {
        let src = match (ird >> 3) & 7 {
            0 => cpu.regs.d[reg as usize],
            1 => cpu.regs.an(reg),
            _ => cpu.lat.val,
        };
        let (r, sr) = exec(op, size, src, cpu.regs.d[dn], cpu.regs.sr);
        if op != Alu::Cmp {
            set_dn(cpu, dn, size, r);
        }
        cpu.regs.sr = sr;
    } else {
        let (r, sr) = exec(op, size, cpu.regs.d[dn], cpu.lat.val, cpu.regs.sr);
        cpu.lat.val = r;
        cpu.regs.sr = sr;
    }
}

const MAXT: usize = 16;

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

const fn push_all(mut e: Entry, p: Plan) -> Entry {
    let mut i = 0;
    while i < p.len() {
        e = push(e, p[i]);
        i += 1;
    }
    e
}

const fn size_of_index(i: usize) -> Size {
    match i {
        0 => Size::Byte,
        1 => Size::Word,
        _ => Size::Long,
    }
}

const fn src_ea(f: usize) -> Ea {
    match f {
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

const fn dst_ea(f: usize) -> Ea {
    match f {
        0 => Ea::Ind(0),
        1 => Ea::PostInc(0),
        2 => Ea::PreDec(0),
        3 => Ea::Disp(0),
        4 => Ea::Index(0),
        5 => Ea::AbsW,
        _ => Ea::AbsL,
    }
}

const SRC_FORMS: usize = 12;
const DST_FORMS: usize = 7;
const N_PLANS: usize = 3 * SRC_FORMS + 3 * DST_FORMS;

const fn build_to_dn(s: usize, f: usize) -> Entry {
    let size = size_of_index(s);
    let long = s == 2;
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), size));
    }
    e = push(e, Tok::H(HK_BINOP));
    e = push(e, Tok::Pf);
    if long {
        // register and immediate sources: `np nn`; memory sources: `np n`
        e = push(e, Tok::N(if f < 2 || f == 11 { 4 } else { 2 }));
    }
    e
}

const fn build_to_ea(s: usize, f: usize) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push_all(e, ea_read_plan(dst_ea(f), size));
    e = push(e, Tok::H(HK_BINOP));
    e = push(e, Tok::Pf);
    push_write(e, s)
}

const fn build_plans() -> [Entry; N_PLANS] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_PLANS];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < SRC_FORMS {
            out[s * SRC_FORMS + f] = build_to_dn(s, f);
            f += 1;
        }
        let mut f = 0;
        while f < DST_FORMS {
            out[3 * SRC_FORMS + s * DST_FORMS + f] = build_to_ea(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

static PLANS: [Entry; N_PLANS] = build_plans();

fn src_form(ird: u16) -> usize {
    let mode = ((ird >> 3) & 7) as usize;
    if mode < 7 {
        mode
    } else {
        7 + (ird & 7) as usize
    }
}

fn dst_form(ird: u16) -> usize {
    let mode = ((ird >> 3) & 7) as usize;
    if mode < 7 {
        mode - 2
    } else {
        5 + (ird & 7) as usize
    }
}

/// Plan of a two-operand instruction (opcode in the `ird` latch of `cpu`).
pub fn binop_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let s = match (ird >> 6) & 3 {
        0 => 0,
        1 => 1,
        _ => 2,
    };
    let idx = if ird & 0x100 == 0 {
        s * SRC_FORMS + src_form(ird)
    } else {
        3 * SRC_FORMS + s * DST_FORMS + dst_form(ird)
    };
    let e = &PLANS[idx];
    &e.toks[..e.n]
}

/// Number of rules of an instruction with both directions (`ADD`, `SUB`, `AND`, `OR`).
pub const N_RULES_BOTH: usize = 35 + 21;
/// Number of rules of an instruction with only the `<ea>,Dn` direction (`CMP`).
pub const N_RULES_TO_DN: usize = 35;

/// Decoding rules of a two-operand instruction: `line` is the top nibble of the opcode
/// (e.g. `0xD000` for ADD). `to_ea` adds the `Dn,<ea>` direction.
pub const fn binop_rules<const N: usize>(line: u16, id: InsnId, to_ea: bool) -> [Rule; N] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N];
    let mut k = 0;
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < SRC_FORMS {
            // no byte operation from An
            if !(s == 0 && f == 1) {
                let (m, v): (u16, u16) = match f {
                    0 => (0x38, 0),
                    1 => (0x38, 1 << 3),
                    2 => (0x38, 2 << 3),
                    3 => (0x38, 3 << 3),
                    4 => (0x38, 4 << 3),
                    5 => (0x38, 5 << 3),
                    6 => (0x38, 6 << 3),
                    7 => (0x3F, 0x38),
                    8 => (0x3F, 0x39),
                    9 => (0x3F, 0x3A),
                    10 => (0x3F, 0x3B),
                    _ => (0x3F, 0x3C),
                };
                out[k] = Rule {
                    mask: 0xF1C0 | m,
                    value: line | ((s as u16) << 6) | v,
                    id,
                    variant: 0,
                };
                k += 1;
            }
            f += 1;
        }
        s += 1;
    }
    if to_ea {
        let mut s = 0;
        while s < 3 {
            let mut f = 0;
            while f < DST_FORMS {
                let (m, v): (u16, u16) = match f {
                    0 => (0x38, 2 << 3),
                    1 => (0x38, 3 << 3),
                    2 => (0x38, 4 << 3),
                    3 => (0x38, 5 << 3),
                    4 => (0x38, 6 << 3),
                    5 => (0x3F, 0x38),
                    _ => (0x3F, 0x39),
                };
                out[k] = Rule {
                    mask: 0xF1C0 | m,
                    value: line | 0x100 | ((s as u16) << 6) | v,
                    id,
                    variant: 1,
                };
                k += 1;
                f += 1;
            }
            s += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Immediate (`ADDI`-style) and quick (`ADDQ`-style) forms. Timing: docs/specs/instr/ADD.md.
// ---------------------------------------------------------------------------------------------

/// Hook of the immediate plans: operation `#imm,<ea>` (immediate saved in `aux`).
pub const HK_IMMOP: u8 = 0x81;
/// Hook of the quick plans: operation `#q,<ea>` (q = bits 11-9, 0 meaning 8).
pub const HK_QOP: u8 = 0x82;
/// Hook saving the immediate operand (`val`) in `aux`.
pub const HK_SAVE: u8 = 0x83;
/// Hook shifting the operand to emit its high word (long writes go low word first).
pub const HK_SHIFT16: u8 = 0x84;

/// Semantics of [`HK_IMMOP`].
pub fn imm_exec(op: Alu, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of_ird(ird);
    let reg = (ird & 7) as usize;
    let src = cpu.lat.aux;
    if (ird >> 3) & 7 == 0 {
        let (r, sr) = exec(op, size, src, cpu.regs.d[reg], cpu.regs.sr);
        if op != Alu::Cmp {
            set_dn(cpu, reg, size, r);
        }
        cpu.regs.sr = sr;
    } else {
        let (r, sr) = exec(op, size, src, cpu.lat.val, cpu.regs.sr);
        cpu.lat.val = r;
        cpu.regs.sr = sr;
    }
}

/// Semantics of [`HK_QOP`]: `ADDQ` / `SUBQ` (never `Cmp` or logic operations).
pub fn quick_exec(op: Alu, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of_ird(ird);
    let reg = (ird & 7) as usize;
    let q = match (ird >> 9) & 7 {
        0 => 8,
        n => n as u32,
    };
    match (ird >> 3) & 7 {
        0 => {
            let (r, sr) = exec(op, size, q, cpu.regs.d[reg], cpu.regs.sr);
            set_dn(cpu, reg, size, r);
            cpu.regs.sr = sr;
        }
        1 => {
            // address register: whole register, flags untouched
            let a = cpu.regs.an(reg as u8);
            let r = if op == Alu::Add {
                a.wrapping_add(q)
            } else {
                a.wrapping_sub(q)
            };
            cpu.regs.set_an(reg as u8, r);
        }
        _ => {
            let (r, sr) = exec(op, size, q, cpu.lat.val, cpu.regs.sr);
            cpu.lat.val = r;
            cpu.regs.sr = sr;
        }
    }
}

const IMM_FORMS: usize = 8;
const Q_FORMS: usize = 9;

const fn imm_ea(f: usize) -> Ea {
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

const fn q_ea(f: usize) -> Ea {
    match f {
        0 => Ea::DataReg(0),
        1 => Ea::AddrReg(0),
        _ => imm_ea(f - 1),
    }
}

const fn push_write(mut e: Entry, s: usize) -> Entry {
    match s {
        0 => push(e, Tok::Wb(AddrSrc::Ea)),
        1 => push(e, Tok::Ww(AddrSrc::Ea)),
        _ => {
            e = push(e, Tok::Ww(AddrSrc::EaPlus2));
            e = push(e, Tok::H(HK_SHIFT16));
            push(e, Tok::Ww(AddrSrc::Ea))
        }
    }
}

const fn build_imm(s: usize, f: usize) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push_all(e, ea_read_plan(Ea::Imm, size));
    e = push(e, Tok::H(HK_SAVE));
    if f > 0 {
        e = push_all(e, ea_read_plan(imm_ea(f), size));
    }
    e = push(e, Tok::H(HK_IMMOP));
    e = push(e, Tok::Pf);
    if f == 0 {
        if s == 2 {
            e = push(e, Tok::N(4));
        }
        e
    } else {
        push_write(e, s)
    }
}

const fn build_quick(s: usize, f: usize) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(q_ea(f), size));
    }
    e = push(e, Tok::H(HK_QOP));
    e = push(e, Tok::Pf);
    if f < 2 {
        // `np` (Dn byte/word), `np nn` (Dn long, An word/long)
        if s == 2 || f == 1 {
            e = push(e, Tok::N(4));
        }
        e
    } else {
        push_write(e, s)
    }
}

const N_IMM: usize = 3 * IMM_FORMS;
const N_Q: usize = 3 * Q_FORMS;

const fn build_imm_plans() -> [Entry; N_IMM] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_IMM];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            out[s * IMM_FORMS + f] = build_imm(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

const fn build_q_plans() -> [Entry; N_Q] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_Q];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < Q_FORMS {
            out[s * Q_FORMS + f] = build_quick(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

static IMM_PLANS: [Entry; N_IMM] = build_imm_plans();
static Q_PLANS: [Entry; N_Q] = build_q_plans();

fn imm_form(ird: u16) -> usize {
    let mode = ((ird >> 3) & 7) as usize;
    if mode == 0 {
        0
    } else if mode < 7 {
        mode - 1
    } else {
        6 + (ird & 7) as usize
    }
}

fn size_index(ird: u16) -> usize {
    match (ird >> 6) & 3 {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

/// Plan of an instruction of the `ADD` family (regular, immediate or quick form).
pub fn arith_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    match ird >> 12 {
        0 => {
            let e = &IMM_PLANS[size_index(ird) * IMM_FORMS + imm_form(ird)];
            &e.toks[..e.n]
        }
        5 => {
            let mode = ((ird >> 3) & 7) as usize;
            let f = if mode < 7 {
                mode
            } else {
                7 + (ird & 7) as usize
            };
            let e = &Q_PLANS[size_index(ird) * Q_FORMS + f];
            &e.toks[..e.n]
        }
        _ => binop_plan(cpu),
    }
}

/// Executes a hook of the `ADD` family: ALU hooks, else effective-address hooks.
pub fn arith_hook(op: Alu, k: u8, cpu: &mut Cpu) {
    match k {
        HK_BINOP => binop_exec(op, cpu),
        HK_IMMOP => imm_exec(op, cpu),
        HK_QOP => quick_exec(op, cpu),
        HK_SAVE => cpu.lat.aux = cpu.lat.val,
        HK_SHIFT16 => cpu.lat.val >>= 16,
        _ => {
            let ird = cpu.lat.ird;
            crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size_of_ird(ird));
        }
    }
}

/// Number of rules of `ADD` / `SUB` with their immediate and quick forms.
pub const N_RULES_ARITH: usize = N_RULES_BOTH + 24 + 26;

/// Decoding rules of `ADD` (`line` 0xD000, `imm` 0x0600, `quick` 0x5000) or `SUB` (0x9000,
/// 0x0400, 0x5100).
pub const fn arith_rules(line: u16, imm: u16, quick: u16, id: InsnId) -> [Rule; N_RULES_ARITH] {
    let none = Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    };
    let mut out = [none; N_RULES_ARITH];
    let both: [Rule; N_RULES_BOTH] = binop_rules(line, id, true);
    let mut k = 0;
    while k < N_RULES_BOTH {
        out[k] = both[k];
        k += 1;
    }
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            let (m, v): (u16, u16) = match f {
                0 => (0x38, 0),
                1 => (0x38, 2 << 3),
                2 => (0x38, 3 << 3),
                3 => (0x38, 4 << 3),
                4 => (0x38, 5 << 3),
                5 => (0x38, 6 << 3),
                6 => (0x3F, 0x38),
                _ => (0x3F, 0x39),
            };
            out[k] = Rule {
                mask: 0xFFC0 | m,
                value: imm | ((s as u16) << 6) | v,
                id,
                variant: 2,
            };
            k += 1;
            f += 1;
        }
        s += 1;
    }
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < Q_FORMS {
            // no byte quick operation on An
            if !(s == 0 && f == 1) {
                let (m, v): (u16, u16) = match f {
                    0 => (0x38, 0),
                    1 => (0x38, 1 << 3),
                    2 => (0x38, 2 << 3),
                    3 => (0x38, 3 << 3),
                    4 => (0x38, 4 << 3),
                    5 => (0x38, 5 << 3),
                    6 => (0x38, 6 << 3),
                    7 => (0x3F, 0x38),
                    _ => (0x3F, 0x39),
                };
                out[k] = Rule {
                    mask: 0xF1C0 | m,
                    value: quick | ((s as u16) << 6) | v,
                    id,
                    variant: 3,
                };
                k += 1;
            }
            f += 1;
        }
        s += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e07_00_add_flags() {
        // 0x7F + 1 = 0x80: N, V, no carry (byte)
        let (r, c, v) = add_flags(Size::Byte, 1, 0x7F, 0);
        assert_eq!((r, c, v), (0x80, false, true));
        // 0xFF + 1 = 0: carry, Z
        let (r, c, v) = add_flags(Size::Byte, 1, 0xFF, 0);
        assert_eq!((r, c, v), (0, true, false));
    }

    #[test]
    fn e07_00_sub_flags() {
        // 0 - 1: borrow, negative
        let (r, c, v) = sub_flags(Size::Word, 1, 0, 0);
        assert_eq!((r, c, v), (0xFFFF, true, false));
        // 0x80 - 1 (byte): overflow
        let (r, c, v) = sub_flags(Size::Byte, 1, 0x80, 0);
        assert_eq!((r, c, v), (0x7F, false, true));
    }

    #[test]
    fn e07_00_exec_cmp_keeps_x() {
        let (_, sr) = exec(Alu::Cmp, Size::Long, 1, 1, X);
        assert_eq!(sr & (X | Z | N | V | C), X | Z);
    }
}
