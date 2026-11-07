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
    } else if (ird >> 3) & 7 == 0 {
        // `EOR Dn,Dm`: register destination
        let (r, sr) = exec(
            op,
            size,
            cpu.regs.d[dn],
            cpu.regs.d[reg as usize],
            cpu.regs.sr,
        );
        set_dn(cpu, reg as usize, size, r);
        cpu.regs.sr = sr;
    } else {
        let (r, sr) = exec(op, size, cpu.regs.d[dn], cpu.lat.val, cpu.regs.sr);
        cpu.lat.val = r;
        cpu.regs.sr = sr;
    }
}

const MAXT: usize = 24;

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

// ---------------------------------------------------------------------------------------------
// Address register operations (`ADDA` / `SUBA`) and extended operations (`ADDX` / `SUBX`).
// ---------------------------------------------------------------------------------------------

/// Hook of the `ADDA` / `SUBA` plans.
pub const HK_ADDA: u8 = 0x85;
/// Hook of the `ADDX` / `SUBX` plans.
pub const HK_XOP: u8 = 0x86;
/// Hook keeping the word just read (low half of a long read low-word first) in `aux2`.
pub const HK_LO: u8 = 0x87;
/// Hook joining the high word just read with `aux2` into `aux` (source operand).
pub const HK_JOIN_SRC: u8 = 0x88;
/// Hook joining the high word just read with `aux2` into `val` (destination operand).
pub const HK_JOIN_DST: u8 = 0x89;

/// Semantics of [`HK_ADDA`]: `An = An +/- sign-extended source`; no flags.
pub fn adda_exec(op: Alu, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let reg = (ird & 7) as u8;
    let src = match (ird >> 3) & 7 {
        0 => cpu.regs.d[reg as usize],
        1 => cpu.regs.an(reg),
        _ => cpu.lat.val,
    };
    let src = if ird & 0x100 == 0 {
        src as u16 as i16 as i32 as u32
    } else {
        src
    };
    let an = ((ird >> 9) & 7) as u8;
    let a = cpu.regs.an(an);
    let r = if op == Alu::Add {
        a.wrapping_add(src)
    } else {
        a.wrapping_sub(src)
    };
    cpu.regs.set_an(an, r);
}

/// Semantics of [`HK_XOP`]: `dst +/- src +/- X`; Z is only cleared, never set.
pub fn x_exec(op: Alu, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of_ird(ird);
    let regs_only = ird & 8 == 0;
    let (src, dst) = if regs_only {
        (
            cpu.regs.d[(ird & 7) as usize],
            cpu.regs.d[((ird >> 9) & 7) as usize],
        )
    } else {
        (cpu.lat.aux, cpu.lat.val)
    };
    let x = u32::from(cpu.regs.sr & X != 0);
    let (r, c, v) = if op == Alu::Add {
        add_flags(size, src, dst, x)
    } else {
        sub_flags(size, src, dst, x)
    };
    let mut sr = cpu.regs.sr & !(X | N | V | C);
    if r & size_msb(size) != 0 {
        sr |= N;
    }
    if v {
        sr |= V;
    }
    if c {
        sr |= C | X;
    }
    if r & size_mask(size) != 0 {
        sr &= !Z;
    }
    cpu.regs.sr = sr;
    if regs_only {
        set_dn(cpu, ((ird >> 9) & 7) as usize, size, r);
    } else {
        cpu.lat.val = r;
    }
}

const fn build_adda(s: usize, f: usize) -> Entry {
    let size = if s == 0 { Size::Word } else { Size::Long };
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), size));
    }
    e = push(e, Tok::H(HK_ADDA));
    e = push(e, Tok::Pf);
    // word: always `np nn`; long: memory `np n`, register and immediate `np nn`
    push(e, Tok::N(if s == 1 && f >= 2 && f != 11 { 2 } else { 4 }))
}

const N_ADDA: usize = 2 * SRC_FORMS;

const fn build_adda_plans() -> [Entry; N_ADDA] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_ADDA];
    let mut s = 0;
    while s < 2 {
        let mut f = 0;
        while f < SRC_FORMS {
            out[s * SRC_FORMS + f] = build_adda(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

static ADDA_PLANS: [Entry; N_ADDA] = build_adda_plans();

/// Plan of `ADDA` / `SUBA` (opcode in the `ird` latch of `cpu`).
pub fn adda_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let s = usize::from(ird & 0x100 != 0);
    let e = &ADDA_PLANS[s * SRC_FORMS + src_form(ird)];
    &e.toks[..e.n]
}

/// Number of rules of `ADDA` / `SUBA`.
pub const N_RULES_ADDA: usize = 2 * SRC_FORMS;

/// Decoding rules of `ADDA` (`line` 0xD000) or `SUBA` (0x9000).
pub const fn adda_rules(line: u16, id: InsnId) -> [Rule; N_RULES_ADDA] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_ADDA];
    let mut k = 0;
    let mut s = 0;
    while s < 2 {
        let mut f = 0;
        while f < SRC_FORMS {
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
            let opmode: u16 = if s == 0 { 3 } else { 7 };
            out[k] = Rule {
                mask: 0xF1C0 | m,
                value: line | (opmode << 6) | v,
                id,
                variant: s as u8,
            };
            k += 1;
            f += 1;
        }
        s += 1;
    }
    out
}

/// Marks a hook as acting on the destination register field (bits 11-9) instead of bits 2-0.
pub const DST_FLAG: u8 = 0x40;

const fn push_x_read(mut e: Entry, s: usize, dst: bool) -> Entry {
    let flag = if dst { DST_FLAG } else { 0 };
    e = push(e, Tok::H(crate::ea::HK_AN_PREDEC | flag));
    if !dst {
        e = push(e, Tok::N(2));
    }
    match s {
        0 => push(e, Tok::Rb(AddrSrc::Ea)),
        1 => push(e, Tok::Rw(AddrSrc::Ea)),
        _ => {
            // a long is read low word first (SST)
            e = push(e, Tok::Rw(AddrSrc::EaPlus2));
            e = push(e, Tok::H(HK_LO));
            e = push(e, Tok::Rw(AddrSrc::Ea));
            push(e, Tok::H(if dst { HK_JOIN_DST } else { HK_JOIN_SRC }))
        }
    }
}

const fn build_x(s: usize, mem: bool) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if !mem {
        e = push(e, Tok::H(HK_XOP));
        e = push(e, Tok::Pf);
        if s == 2 {
            e = push(e, Tok::N(4));
        }
        return e;
    }
    e = push_x_read(e, s, false);
    if s < 2 {
        e = push(e, Tok::H(HK_SAVE));
    }
    e = push_x_read(e, s, true);
    e = push(e, Tok::H(HK_XOP));
    match s {
        0 => {
            e = push(e, Tok::Pf);
            push(e, Tok::Wb(AddrSrc::Ea))
        }
        1 => {
            e = push(e, Tok::Pf);
            push(e, Tok::Ww(AddrSrc::Ea))
        }
        _ => {
            e = push(e, Tok::Ww(AddrSrc::EaPlus2));
            e = push(e, Tok::Pf);
            e = push(e, Tok::H(HK_SHIFT16));
            push(e, Tok::Ww(AddrSrc::Ea))
        }
    }
}

const fn build_x_plans() -> [Entry; 6] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; 6];
    let mut s = 0;
    while s < 3 {
        out[s * 2] = build_x(s, false);
        out[s * 2 + 1] = build_x(s, true);
        s += 1;
    }
    out
}

static X_PLANS: [Entry; 6] = build_x_plans();

/// Plan of `ADDX` / `SUBX` (opcode in the `ird` latch of `cpu`).
pub fn x_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let e = &X_PLANS[size_index(ird) * 2 + usize::from(ird & 8 != 0)];
    &e.toks[..e.n]
}

/// Hook dispatcher of `ADDX` / `SUBX`.
pub fn x_hook(op: Alu, k: u8, cpu: &mut Cpu) {
    match k {
        HK_XOP => x_exec(op, cpu),
        HK_SAVE => cpu.lat.aux = cpu.lat.val,
        HK_LO => cpu.lat.aux2 = cpu.lat.val & 0xFFFF,
        HK_JOIN_SRC => cpu.lat.aux = (cpu.lat.val << 16) | cpu.lat.aux2,
        HK_JOIN_DST => cpu.lat.val = (cpu.lat.val << 16) | cpu.lat.aux2,
        HK_SHIFT16 => cpu.lat.val >>= 16,
        _ => {
            let ird = cpu.lat.ird;
            let size = size_of_ird(ird);
            if k & DST_FLAG != 0 {
                crate::ea::run_ea_hook(cpu, k & !DST_FLAG, ((ird >> 9) & 7) as u8, size);
            } else {
                crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size);
            }
        }
    }
}

/// Hook dispatcher of `ADDA` / `SUBA`.
pub fn adda_hook(op: Alu, k: u8, cpu: &mut Cpu) {
    if k == HK_ADDA {
        adda_exec(op, cpu);
    } else {
        let ird = cpu.lat.ird;
        let size = if ird & 0x100 == 0 {
            Size::Word
        } else {
            Size::Long
        };
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size);
    }
}

/// Number of rules of `ADDX` / `SUBX`.
pub const N_RULES_X: usize = 6;

/// Decoding rules of `ADDX` (`line` 0xD000) or `SUBX` (0x9000): `line | Dx << 9 | 1 << 8 |
/// size << 6 | m << 3 | Dy`.
pub const fn x_rules(line: u16, id: InsnId) -> [Rule; N_RULES_X] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_X];
    let mut k = 0;
    let mut s = 0;
    while s < 3 {
        let mut m = 0;
        while m < 2 {
            out[k] = Rule {
                mask: 0xF1F8,
                value: line | 0x100 | ((s as u16) << 6) | ((m as u16) << 3),
                id,
                variant: m as u8,
            };
            k += 1;
            m += 1;
        }
        s += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// One-operand instructions on a data-alterable operand: NEG, NEGX, NOT, TST.
// Timing: docs/specs/instr/CLR.md ("CLR, NEGX, NEG, NOT" table) and TST.md.
// ---------------------------------------------------------------------------------------------

/// One-operand operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unary {
    /// `0 - dst`.
    Neg,
    /// `0 - dst - X`.
    Negx,
    /// `!dst`.
    Not,
    /// Flags of `dst` only.
    Tst,
}

/// Hook of the one-operand plans.
pub const HK_UNARY: u8 = 0x8A;

/// Semantics of [`HK_UNARY`].
pub fn unary_exec(op: Unary, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let size = size_of_ird(ird);
    let reg = (ird & 7) as usize;
    let reg_dst = (ird >> 3) & 7 == 0;
    let x = if reg_dst {
        cpu.regs.d[reg]
    } else {
        cpu.lat.val
    };
    let mut sr = cpu.regs.sr;
    let r = match op {
        Unary::Neg | Unary::Negx => {
            let bin = if op == Unary::Negx {
                u32::from(sr & X != 0)
            } else {
                0
            };
            let (r, c, v) = sub_flags(size, x, 0, bin);
            sr &= !(X | N | V | C);
            if r & size_msb(size) != 0 {
                sr |= N;
            }
            if v {
                sr |= V;
            }
            if c {
                sr |= C | X;
            }
            if op == Unary::Neg {
                if r & size_mask(size) == 0 {
                    sr |= Z;
                } else {
                    sr &= !Z;
                }
            } else if r & size_mask(size) != 0 {
                sr &= !Z;
            }
            r
        }
        Unary::Not | Unary::Tst => {
            let r = if op == Unary::Not { !x } else { x } & size_mask(size);
            sr &= !(N | Z | V | C);
            sr |= nz_flags(size, r);
            r
        }
    };
    cpu.regs.sr = sr;
    if op != Unary::Tst {
        if reg_dst {
            set_dn(cpu, reg, size, r);
        } else {
            cpu.lat.val = r;
        }
    }
}

const fn build_unary(s: usize, f: usize, write: bool) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f > 0 {
        e = push_all(e, ea_read_plan(imm_ea(f), size));
    }
    e = push(e, Tok::H(HK_UNARY));
    e = push(e, Tok::Pf);
    if f == 0 {
        // long on a register: `np n` (TST has no extra cycle)
        if s == 2 && write {
            e = push(e, Tok::N(2));
        }
        e
    } else if write {
        push_write(e, s)
    } else {
        e
    }
}

const N_UNARY: usize = 3 * IMM_FORMS;

const fn build_unary_plans(write: bool) -> [Entry; N_UNARY] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_UNARY];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            out[s * IMM_FORMS + f] = build_unary(s, f, write);
            f += 1;
        }
        s += 1;
    }
    out
}

static UNARY_RMW: [Entry; N_UNARY] = build_unary_plans(true);
static UNARY_TST: [Entry; N_UNARY] = build_unary_plans(false);

/// Plan of `NEG` / `NEGX` / `NOT` (read-modify-write) or `TST` (`test` = true).
pub fn unary_plan(cpu: &Cpu, test: bool) -> Plan {
    let ird = cpu.lat.ird;
    let t = if test { &UNARY_TST } else { &UNARY_RMW };
    let e = &t[size_index(ird) * IMM_FORMS + imm_form(ird)];
    &e.toks[..e.n]
}

/// Hook dispatcher of the one-operand instructions.
pub fn unary_hook(op: Unary, k: u8, cpu: &mut Cpu) {
    match k {
        HK_UNARY => unary_exec(op, cpu),
        HK_SHIFT16 => cpu.lat.val >>= 16,
        _ => {
            let ird = cpu.lat.ird;
            crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size_of_ird(ird));
        }
    }
}

/// Number of rules of a one-operand instruction.
pub const N_RULES_UNARY: usize = 3 * IMM_FORMS;

/// Decoding rules of `base | size << 6 | ea` over the data-alterable modes.
pub const fn unary_rules(base: u16, id: InsnId) -> [Rule; N_RULES_UNARY] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_UNARY];
    let mut k = 0;
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
                value: base | ((s as u16) << 6) | v,
                id,
                variant: 0,
            };
            k += 1;
            f += 1;
        }
        s += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Comparisons: CMP, CMPI, CMPM, CMPA. Timing: docs/specs/instr/CMP.md and CMPA.md.
// ---------------------------------------------------------------------------------------------

/// Hook of the `CMPM` plans.
pub const HK_CMPM: u8 = 0x8B;
/// Hook of the `CMPA` plans.
pub const HK_CMPA: u8 = 0x8C;

/// Semantics of [`HK_CMPM`]: `(Ax)+ - (Ay)+` flags (source in `aux`, destination in `val`).
pub fn cmpm_exec(cpu: &mut Cpu) {
    let size = size_of_ird(cpu.lat.ird);
    let (_, sr) = exec(Alu::Cmp, size, cpu.lat.aux, cpu.lat.val, cpu.regs.sr);
    cpu.regs.sr = sr;
}

/// Semantics of [`HK_CMPA`]: `An - source` (word sign-extended) flags on 32 bits.
pub fn cmpa_exec(cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let reg = (ird & 7) as u8;
    let src = match (ird >> 3) & 7 {
        0 => cpu.regs.d[reg as usize],
        1 => cpu.regs.an(reg),
        _ => cpu.lat.val,
    };
    let src = if ird & 0x100 == 0 {
        src as u16 as i16 as i32 as u32
    } else {
        src
    };
    let dst = cpu.regs.an(((ird >> 9) & 7) as u8);
    let (_, sr) = exec(Alu::Cmp, Size::Long, src, dst, cpu.regs.sr);
    cpu.regs.sr = sr;
}

const fn build_cmp(s: usize, f: usize) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), size));
    }
    e = push(e, Tok::H(HK_BINOP));
    e = push(e, Tok::Pf);
    if s == 2 {
        e = push(e, Tok::N(2));
    }
    e
}

const fn build_cmpi(s: usize, f: usize) -> Entry {
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
    if f == 0 && s == 2 {
        e = push(e, Tok::N(2));
    }
    e
}

const fn push_flagged(mut e: Entry, p: Plan) -> Entry {
    let mut i = 0;
    while i < p.len() {
        e = push(
            e,
            match p[i] {
                Tok::H(k) => Tok::H(k | DST_FLAG),
                t => t,
            },
        );
        i += 1;
    }
    e
}

const fn build_cmpm(s: usize) -> Entry {
    let size = size_of_index(s);
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push_all(e, ea_read_plan(Ea::PostInc(0), size));
    e = push(e, Tok::H(HK_SAVE));
    e = push_flagged(e, ea_read_plan(Ea::PostInc(0), size));
    e = push(e, Tok::H(HK_CMPM));
    push(e, Tok::Pf)
}

const fn build_cmpa(s: usize, f: usize) -> Entry {
    let size = if s == 0 { Size::Word } else { Size::Long };
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), size));
    }
    e = push(e, Tok::H(HK_CMPA));
    e = push(e, Tok::Pf);
    push(e, Tok::N(2))
}

const fn build_cmp_plans() -> [Entry; 3 * SRC_FORMS] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; 3 * SRC_FORMS];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < SRC_FORMS {
            out[s * SRC_FORMS + f] = build_cmp(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

const fn build_cmpi_plans() -> [Entry; N_IMM] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_IMM];
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            out[s * IMM_FORMS + f] = build_cmpi(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

const fn build_cmpa_plans() -> [Entry; N_ADDA] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; N_ADDA];
    let mut s = 0;
    while s < 2 {
        let mut f = 0;
        while f < SRC_FORMS {
            out[s * SRC_FORMS + f] = build_cmpa(s, f);
            f += 1;
        }
        s += 1;
    }
    out
}

static CMP_PLANS: [Entry; 3 * SRC_FORMS] = build_cmp_plans();
static CMPI_PLANS: [Entry; N_IMM] = build_cmpi_plans();
static CMPM_PLANS: [Entry; 3] = [build_cmpm(0), build_cmpm(1), build_cmpm(2)];
static CMPA_PLANS: [Entry; N_ADDA] = build_cmpa_plans();

/// Plan of `CMP`, `CMPI` or `CMPM` (opcode in the `ird` latch of `cpu`).
pub fn cmp_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let e = if ird >> 12 == 0 {
        &CMPI_PLANS[size_index(ird) * IMM_FORMS + imm_form(ird)]
    } else if ird & 0x138 == 0x108 {
        &CMPM_PLANS[size_index(ird)]
    } else {
        &CMP_PLANS[size_index(ird) * SRC_FORMS + src_form(ird)]
    };
    &e.toks[..e.n]
}

/// Plan of `CMPA`.
pub fn cmpa_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let s = usize::from(ird & 0x100 != 0);
    let e = &CMPA_PLANS[s * SRC_FORMS + src_form(ird)];
    &e.toks[..e.n]
}

/// Hook dispatcher of `CMP`, `CMPI` and `CMPM`.
pub fn cmp_hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_BINOP => binop_exec(Alu::Cmp, cpu),
        HK_IMMOP => imm_exec(Alu::Cmp, cpu),
        HK_CMPM => cmpm_exec(cpu),
        HK_SAVE => cpu.lat.aux = cpu.lat.val,
        _ => {
            let ird = cpu.lat.ird;
            let size = size_of_ird(ird);
            if k & 0xC0 == DST_FLAG {
                crate::ea::run_ea_hook(cpu, k & !DST_FLAG, ((ird >> 9) & 7) as u8, size);
            } else {
                crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size);
            }
        }
    }
}

/// Hook dispatcher of `CMPA`.
pub fn cmpa_hook(k: u8, cpu: &mut Cpu) {
    if k == HK_CMPA {
        cmpa_exec(cpu);
    } else {
        let ird = cpu.lat.ird;
        let size = if ird & 0x100 == 0 {
            Size::Word
        } else {
            Size::Long
        };
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, size);
    }
}

/// Number of rules of `CMP` with `CMPI` and `CMPM`.
pub const N_RULES_CMP: usize = N_RULES_TO_DN + 24 + 3;

/// Decoding rules of `CMP` (`0xB000`), `CMPI` (`0x0C00`) and `CMPM`.
pub const fn cmp_rules(id: InsnId) -> [Rule; N_RULES_CMP] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_CMP];
    let plain: [Rule; N_RULES_TO_DN] = binop_rules(0xB000, id, false);
    let mut k = 0;
    while k < N_RULES_TO_DN {
        out[k] = plain[k];
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
                value: 0x0C00 | ((s as u16) << 6) | v,
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
        out[k] = Rule {
            mask: 0xF1F8,
            value: 0xB108 | ((s as u16) << 6),
            id,
            variant: 3,
        };
        k += 1;
        s += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// MULU / MULS. Timing: docs/specs/instr/MULU.md: `np` then 17 + m internal cycles (`Tok::Nv`).
// ---------------------------------------------------------------------------------------------

/// Hook of the `MULU` / `MULS` plans (computes the product and the length of the idle cycle).
pub const HK_MUL: u8 = 0x8D;

/// Number of 01 / 10 patterns in the 16-bit source with a zero appended (`MULS`).
pub fn muls_pattern_count(src: u16) -> u32 {
    let v = (src as u32) << 1;
    ((v ^ (v >> 1)) & 0xFFFF).count_ones()
}

/// Semantics of [`HK_MUL`].
pub fn mul_exec(signed: bool, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let reg = (ird & 7) as usize;
    let src = if (ird >> 3) & 7 == 0 {
        cpu.regs.d[reg] as u16
    } else {
        cpu.lat.val as u16
    };
    let dn = ((ird >> 9) & 7) as usize;
    let dst = cpu.regs.d[dn] as u16;
    let (r, m) = if signed {
        (
            (i32::from(src as i16) * i32::from(dst as i16)) as u32,
            muls_pattern_count(src),
        )
    } else {
        (u32::from(src) * u32::from(dst), src.count_ones())
    };
    cpu.regs.d[dn] = r;
    cpu.lat.idle = (34 + 2 * m) as u8;
    let mut sr = cpu.regs.sr & !(N | Z | V | C);
    if r == 0 {
        sr |= Z;
    }
    if r & 0x8000_0000 != 0 {
        sr |= N;
    }
    cpu.regs.sr = sr;
}

const fn build_mul(f: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), Size::Word));
    }
    e = push(e, Tok::H(HK_MUL));
    e = push(e, Tok::Pf);
    push(e, Tok::Nv)
}

const fn build_mul_plans() -> [Entry; SRC_FORMS] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; SRC_FORMS];
    let mut f = 0;
    while f < SRC_FORMS {
        out[f] = build_mul(f);
        f += 1;
    }
    out
}

static MUL_PLANS: [Entry; SRC_FORMS] = build_mul_plans();

/// Plan of `MULU` / `MULS`.
pub fn mul_plan(cpu: &Cpu) -> Plan {
    let e = &MUL_PLANS[src_form(cpu.lat.ird)];
    &e.toks[..e.n]
}

/// Hook dispatcher of `MULU` / `MULS`.
pub fn mul_hook(signed: bool, k: u8, cpu: &mut Cpu) {
    if k == HK_MUL {
        mul_exec(signed, cpu);
    } else {
        let ird = cpu.lat.ird;
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, Size::Word);
    }
}

/// Number of rules of `MULU` / `MULS` (every mode but `An`).
pub const N_RULES_MUL: usize = SRC_FORMS - 1;

/// Decoding rules of `MULU` (`opmode` 0xC0C0) or `MULS` (0xC1C0): `base | Dn << 9 | ea`.
pub const fn mul_rules(base: u16, id: InsnId) -> [Rule; N_RULES_MUL] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_MUL];
    let mut k = 0;
    let mut f = 0;
    while f < SRC_FORMS {
        if f != 1 {
            let (m, v): (u16, u16) = match f {
                0 => (0x38, 0),
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
                value: base | v,
                id,
                variant: 0,
            };
            k += 1;
        }
        f += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// DIVU / DIVS. Timing: docs/specs/instr/DIVU.md and DIVS.md (variable idle cycle, `Tok::Nv`).
// ---------------------------------------------------------------------------------------------

/// Hook of the `DIVU` / `DIVS` plans.
pub const HK_DIV: u8 = 0x8E;

/// Total clocks of `DIVU` (without the effective address) for a dividend and a non-zero divisor:
/// 10 on overflow, else 76 + 2 per iteration whose partial dividend has its MSB clear (minus 1 when
/// the subtraction then succeeds).
pub fn divu_clocks(dividend: u32, divisor: u16) -> u32 {
    if (dividend >> 16) >= u32::from(divisor) {
        return 10;
    }
    let mut mcycles = 38;
    let hdiv = u32::from(divisor) << 16;
    let mut d = dividend;
    for _ in 0..15 {
        if (d as i32) < 0 {
            d = d.wrapping_shl(1).wrapping_sub(hdiv);
        } else {
            d = d.wrapping_shl(1);
            mcycles += 2;
            if d >= hdiv {
                d = d.wrapping_sub(hdiv);
                mcycles -= 1;
            }
        }
    }
    mcycles * 2
}

/// Total clocks of `DIVS` (without the effective address) for a non-zero divisor.
pub fn divs_clocks(dividend: i32, divisor: i16) -> u32 {
    let mut mcycles = 6;
    if dividend < 0 {
        mcycles += 1;
    }
    let adividend = dividend.unsigned_abs();
    let adivisor = u32::from(divisor.unsigned_abs());
    if (adividend >> 16) >= adivisor {
        return (mcycles + 2) * 2;
    }
    let mut aquot = adividend / adivisor;
    mcycles += 55;
    if divisor >= 0 {
        if dividend >= 0 {
            mcycles -= 1;
        } else {
            mcycles += 1;
        }
    }
    for _ in 0..15 {
        if (aquot as u16 as i16) >= 0 {
            mcycles += 1;
        }
        aquot <<= 1;
    }
    mcycles * 2
}

/// Semantics of [`HK_DIV`]. Division by zero (an exception, E11) leaves the operands alone.
pub fn div_exec(signed: bool, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let reg = (ird & 7) as usize;
    let src = if (ird >> 3) & 7 == 0 {
        cpu.regs.d[reg] as u16
    } else {
        cpu.lat.val as u16
    };
    let dn = ((ird >> 9) & 7) as usize;
    let dividend = cpu.regs.d[dn];
    cpu.lat.idle = 0;
    if src == 0 {
        return;
    }
    let mut sr = cpu.regs.sr & !(N | Z | V | C);
    let total;
    if signed {
        let (dd, dv) = (dividend as i32, src as i16);
        total = divs_clocks(dd, dv);
        let q = i64::from(dd) / i64::from(dv);
        if !(-0x8000..=0x7FFF).contains(&q) {
            sr |= V | N;
        } else {
            let r = i64::from(dd) % i64::from(dv);
            cpu.regs.d[dn] = ((r as u32 & 0xFFFF) << 16) | (q as u32 & 0xFFFF);
            if q == 0 {
                sr |= Z;
            }
            if q < 0 {
                sr |= N;
            }
        }
    } else {
        total = divu_clocks(dividend, src);
        let dv = u32::from(src);
        if (dividend >> 16) >= dv {
            sr |= V | N;
        } else {
            let (q, r) = (dividend / dv, dividend % dv);
            cpu.regs.d[dn] = (r << 16) | q;
            if q == 0 {
                sr |= Z;
            }
            if q & 0x8000 != 0 {
                sr |= N;
            }
        }
    }
    cpu.regs.sr = sr;
    // the final `np` is a bus cycle of the plan
    cpu.lat.idle = (total - 4) as u8;
}

const fn build_div(f: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), Size::Word));
    }
    e = push(e, Tok::H(HK_DIV));
    e = push(e, Tok::Nv);
    push(e, Tok::Pf)
}

const fn build_div_plans() -> [Entry; SRC_FORMS] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; SRC_FORMS];
    let mut f = 0;
    while f < SRC_FORMS {
        out[f] = build_div(f);
        f += 1;
    }
    out
}

static DIV_PLANS: [Entry; SRC_FORMS] = build_div_plans();

/// Plan of `DIVU` / `DIVS`.
pub fn div_plan(cpu: &Cpu) -> Plan {
    let e = &DIV_PLANS[src_form(cpu.lat.ird)];
    &e.toks[..e.n]
}

/// Hook dispatcher of `DIVU` / `DIVS`.
pub fn div_hook(signed: bool, k: u8, cpu: &mut Cpu) {
    if k == HK_DIV {
        div_exec(signed, cpu);
    } else {
        let ird = cpu.lat.ird;
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, Size::Word);
    }
}

// ---------------------------------------------------------------------------------------------
// AND, OR, EOR (and their immediate forms). Same plans as ADD; EOR also targets a data register.
// ---------------------------------------------------------------------------------------------

const fn build_eor_dn(s: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push(e, Tok::H(HK_BINOP));
    e = push(e, Tok::Pf);
    if s == 2 {
        e = push(e, Tok::N(4));
    }
    e
}

static EOR_DN_PLANS: [Entry; 3] = [build_eor_dn(0), build_eor_dn(1), build_eor_dn(2)];

/// Plan of `AND`, `OR`, `EOR` and their immediate forms.
pub fn logic_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    if ird >> 12 != 0 && ird & 0x100 != 0 && (ird >> 3) & 7 == 0 {
        let e = &EOR_DN_PLANS[size_index(ird)];
        &e.toks[..e.n]
    } else {
        arith_plan(cpu)
    }
}

/// Number of rules of `AND` / `OR` (both directions, no address register) with the immediate form.
pub const N_RULES_AND: usize = 33 + 21 + 24;
/// Number of rules of `EOR` (`Dn,<ea>` with a register or memory destination) with the immediate form.
pub const N_RULES_EOR: usize = 24 + 24;

const fn imm_form_rule(imm: u16, s: usize, f: usize, id: InsnId) -> Rule {
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
    Rule {
        mask: 0xFFC0 | m,
        value: imm | ((s as u16) << 6) | v,
        id,
        variant: 2,
    }
}

/// Decoding rules of `AND` (0xC000, 0x0200) or `OR` (0x8000, 0x0000).
pub const fn and_rules(line: u16, imm: u16, id: InsnId) -> [Rule; N_RULES_AND] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_AND];
    let both: [Rule; N_RULES_BOTH] = binop_rules(line, id, true);
    let mut k = 0;
    let mut j = 0;
    while j < N_RULES_BOTH {
        // no address register source (value bits 5-3 == 001 on the `<ea>,Dn` direction)
        let r = both[j];
        let is_an = r.variant == 0 && r.value & 0x0138 == 0x0008 && r.mask & 0x3F == 0x38;
        if !is_an {
            out[k] = r;
            k += 1;
        }
        j += 1;
    }
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            out[k] = imm_form_rule(imm, s, f, id);
            k += 1;
            f += 1;
        }
        s += 1;
    }
    out
}

/// Decoding rules of `EOR` (0xB100, 0x0A00): `Dn,<ea>` on every data-alterable destination.
pub const fn eor_rules(id: InsnId) -> [Rule; N_RULES_EOR] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_EOR];
    let mut k = 0;
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            let r = imm_form_rule(0xB100, s, f, id);
            out[k] = Rule {
                mask: 0xF1C0 | (r.mask & 0x3F),
                value: r.value,
                id,
                variant: 1,
            };
            k += 1;
            f += 1;
        }
        s += 1;
    }
    let mut s = 0;
    while s < 3 {
        let mut f = 0;
        while f < IMM_FORMS {
            out[k] = imm_form_rule(0x0A00, s, f, id);
            k += 1;
            f += 1;
        }
        s += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Shifts and rotates: ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR.
// Timing: docs/specs/instr/ASL.md. Register forms: `np n* n` (`nn` when long) with m = count;
// memory forms (word, count 1): `nr np nw`.
// ---------------------------------------------------------------------------------------------

/// Hook of the shift plans.
pub const HK_SHIFT: u8 = 0x8F;

/// Shift or rotate operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shift {
    /// Arithmetic shift left.
    Asl,
    /// Arithmetic shift right.
    Asr,
    /// Logical shift left.
    Lsl,
    /// Logical shift right.
    Lsr,
    /// Rotate left.
    Rol,
    /// Rotate right.
    Ror,
    /// Rotate left through X.
    Roxl,
    /// Rotate right through X.
    Roxr,
}

/// Semantics of [`HK_SHIFT`]: result, flags and the length of the variable idle cycle.
pub fn shift_exec(op: Shift, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let mem = (ird >> 6) & 3 == 3;
    let size = if mem { Size::Word } else { size_of_ird(ird) };
    let reg = (ird & 7) as usize;
    let msb = size_msb(size);
    let mask = size_mask(size);
    let mut x = if mem {
        cpu.lat.val & mask
    } else {
        cpu.regs.d[reg] & mask
    };
    let count = if mem {
        1
    } else if ird & 0x20 != 0 {
        cpu.regs.d[((ird >> 9) & 7) as usize] & 63
    } else {
        match (ird >> 9) & 7 {
            0 => 8,
            c => u32::from(c),
        }
    };
    let mut sr = cpu.regs.sr;
    let mut xf = sr & X != 0;
    let mut c = false;
    let mut v = false;
    let mut first = true;
    let mut i = 0;
    while i < count {
        first = false;
        match op {
            Shift::Asl | Shift::Lsl => {
                c = x & msb != 0;
                let n = (x << 1) & mask;
                if op == Shift::Asl && (n & msb) != (x & msb) {
                    v = true;
                }
                x = n;
            }
            Shift::Asr | Shift::Lsr => {
                c = x & 1 != 0;
                let top = if op == Shift::Asr { x & msb } else { 0 };
                x = (x >> 1) | top;
            }
            Shift::Rol => {
                c = x & msb != 0;
                x = ((x << 1) & mask) | u32::from(c);
            }
            Shift::Ror => {
                c = x & 1 != 0;
                x = (x >> 1) | if c { msb } else { 0 };
            }
            Shift::Roxl => {
                c = x & msb != 0;
                x = ((x << 1) & mask) | u32::from(xf);
                xf = c;
            }
            Shift::Roxr => {
                c = x & 1 != 0;
                x = (x >> 1) | if xf { msb } else { 0 };
                xf = c;
            }
        }
        i += 1;
    }
    if first && matches!(op, Shift::Roxl | Shift::Roxr) {
        c = xf;
    }
    if !matches!(op, Shift::Rol | Shift::Ror) && count > 0 {
        xf = c;
    }
    sr &= !(X | N | Z | V | C);
    sr |= nz_flags(size, x);
    if c {
        sr |= C;
    }
    if v {
        sr |= V;
    }
    if xf {
        sr |= X;
    }
    cpu.regs.sr = sr;
    if mem {
        cpu.lat.val = x;
    } else {
        set_dn(cpu, reg, size, x);
        cpu.lat.idle = (2 * count) as u8;
    }
}

const fn build_shift_reg(long: bool) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push(e, Tok::H(HK_SHIFT));
    e = push(e, Tok::Pf);
    e = push(e, Tok::N(if long { 4 } else { 2 }));
    push(e, Tok::Nv)
}

const fn build_shift_mem(f: usize) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    e = push_all(e, ea_read_plan(imm_ea(f), Size::Word));
    e = push(e, Tok::H(HK_SHIFT));
    e = push(e, Tok::Pf);
    push_write(e, 1)
}

const fn build_shift_mems() -> [Entry; 7] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; 7];
    let mut f = 1;
    while f < IMM_FORMS {
        out[f - 1] = build_shift_mem(f);
        f += 1;
    }
    out
}

static SHIFT_REG: [Entry; 2] = [build_shift_reg(false), build_shift_reg(true)];
static SHIFT_MEM: [Entry; 7] = build_shift_mems();

/// Plan of the shift or rotate being executed.
pub fn shift_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let e = if (ird >> 6) & 3 == 3 {
        &SHIFT_MEM[imm_form(ird) - 1]
    } else {
        &SHIFT_REG[usize::from((ird >> 6) & 3 == 2)]
    };
    &e.toks[..e.n]
}

/// Hook dispatcher of the shifts and rotates.
pub fn shift_hook(op: Shift, k: u8, cpu: &mut Cpu) {
    if k == HK_SHIFT {
        shift_exec(op, cpu);
    } else {
        let ird = cpu.lat.ird;
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, Size::Word);
    }
}

/// Number of rules of a shift: 6 register forms and 7 memory forms.
pub const N_RULES_SHIFT: usize = 13;

/// Decoding rules of a shift of type `tt` (0 AS, 1 LS, 2 ROX, 3 RO) and direction `left`.
pub const fn shift_rules(tt: u16, left: bool, id: InsnId) -> [Rule; N_RULES_SHIFT] {
    let d: u16 = if left { 0x100 } else { 0 };
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_SHIFT];
    let mut k = 0;
    let mut s = 0;
    while s < 3 {
        let mut i = 0;
        while i < 2 {
            out[k] = Rule {
                mask: 0xF1F8,
                value: 0xE000 | d | ((s as u16) << 6) | ((i as u16) << 5) | (tt << 3),
                id,
                variant: 0,
            };
            k += 1;
            i += 1;
        }
        s += 1;
    }
    let mut f = 1;
    while f < IMM_FORMS {
        let (m, v): (u16, u16) = match f {
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
            value: 0xE0C0 | (tt << 9) | d | v,
            id,
            variant: 0,
        };
        k += 1;
        f += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Bit operations: BTST, BCHG, BCLR, BSET. Timing: docs/specs/instr/BTST.md, BSET.md, BCLR.md.
// ---------------------------------------------------------------------------------------------

/// Hook of the bit-operation plans.
pub const HK_BIT: u8 = 0x90;

/// Bit operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bit {
    /// `BTST`: test only.
    Test,
    /// `BCHG`: toggle.
    Chg,
    /// `BCLR`: clear.
    Clr,
    /// `BSET`: set.
    Set,
}

/// Semantics of [`HK_BIT`]: Z from the tested bit, new value, idle length of the register forms.
pub fn bit_exec(op: Bit, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let imm = ird & 0x100 == 0;
    let num = if imm {
        cpu.lat.aux
    } else {
        cpu.regs.d[((ird >> 9) & 7) as usize]
    };
    let reg_dst = (ird >> 3) & 7 == 0;
    let reg = (ird & 7) as usize;
    let (n, x) = if reg_dst {
        (num & 31, cpu.regs.d[reg])
    } else {
        (num & 7, cpu.lat.val)
    };
    let bit = 1u32 << n;
    if x & bit == 0 {
        cpu.regs.sr |= Z;
    } else {
        cpu.regs.sr &= !Z;
    }
    let r = match op {
        Bit::Test => x,
        Bit::Chg => x ^ bit,
        Bit::Clr => x & !bit,
        Bit::Set => x | bit,
    };
    if reg_dst {
        cpu.regs.d[reg] = r;
        cpu.lat.idle = match op {
            Bit::Test => 2,
            Bit::Chg | Bit::Set => {
                if n < 16 {
                    2
                } else {
                    4
                }
            }
            Bit::Clr => {
                if n < 16 {
                    4
                } else {
                    6
                }
            }
        };
    } else {
        cpu.lat.val = r;
    }
}

const fn build_bit(imm: bool, f: usize, write: bool) -> Entry {
    let mut e = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    if imm {
        e = push_all(e, ea_read_plan(Ea::Imm, Size::Byte));
        e = push(e, Tok::H(HK_SAVE));
    }
    if f >= 2 {
        e = push_all(e, ea_read_plan(src_ea(f), Size::Byte));
    }
    e = push(e, Tok::H(HK_BIT));
    e = push(e, Tok::Pf);
    if f == 0 {
        push(e, Tok::Nv)
    } else if f == 11 && !imm {
        // `BTST Dn,#<data>`: `np np n`
        push(e, Tok::N(2))
    } else if write {
        push(e, Tok::Wb(AddrSrc::Ea))
    } else {
        e
    }
}

const fn build_bit_plans(write: bool) -> [Entry; 2 * SRC_FORMS] {
    let empty = Entry {
        toks: [Tok::N(0); MAXT],
        n: 0,
    };
    let mut out = [empty; 2 * SRC_FORMS];
    let mut i = 0;
    while i < 2 {
        let mut f = 0;
        while f < SRC_FORMS {
            out[i * SRC_FORMS + f] = build_bit(i == 1, f, write);
            f += 1;
        }
        i += 1;
    }
    out
}

static BIT_RMW: [Entry; 2 * SRC_FORMS] = build_bit_plans(true);
static BIT_TST: [Entry; 2 * SRC_FORMS] = build_bit_plans(false);

/// Plan of a bit operation (`test` = true for `BTST`).
pub fn bit_plan(cpu: &Cpu, test: bool) -> Plan {
    let ird = cpu.lat.ird;
    let t = if test { &BIT_TST } else { &BIT_RMW };
    let i = usize::from(ird & 0x100 == 0);
    let e = &t[i * SRC_FORMS + src_form(ird)];
    &e.toks[..e.n]
}

/// Hook dispatcher of the bit operations.
pub fn bit_hook(op: Bit, k: u8, cpu: &mut Cpu) {
    match k {
        HK_BIT => bit_exec(op, cpu),
        HK_SAVE => cpu.lat.aux = cpu.lat.val,
        _ => {
            let ird = cpu.lat.ird;
            crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, Size::Byte);
        }
    }
}

/// Number of rules of `BTST` (data addressing modes) and of the other bit operations.
pub const N_RULES_BTST: usize = 11 + 10;
/// Number of rules of `BCHG`, `BCLR`, `BSET` (data-alterable modes).
pub const N_RULES_BITMOD: usize = 8 + 8;

const fn bit_mode(f: usize) -> (u16, u16) {
    match f {
        0 => (0x38, 0),
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
    }
}

/// Decoding rules of a bit operation (`tt`: 0 BTST, 1 BCHG, 2 BCLR, 3 BSET) for `N` rules.
pub const fn bit_rules<const N: usize>(tt: u16, id: InsnId) -> [Rule; N] {
    let btst = tt == 0;
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N];
    let mut k = 0;
    let mut form = 0;
    while form < 2 {
        let mut f = 0;
        while f < SRC_FORMS {
            let ok = f != 1 && (f <= 8 || (btst && (f <= 10 || (f == 11 && form == 0))));
            if ok {
                let (m, v) = bit_mode(f);
                out[k] = if form == 0 {
                    Rule {
                        mask: 0xF1C0 | m,
                        value: 0x0100 | (tt << 6) | v,
                        id,
                        variant: 0,
                    }
                } else {
                    Rule {
                        mask: 0xFFC0 | m,
                        value: 0x0800 | (tt << 6) | v,
                        id,
                        variant: 0,
                    }
                };
                k += 1;
            }
            f += 1;
        }
        form += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Decimal arithmetic: ABCD, SBCD, NBCD. Timing: docs/specs/instr/ABCD.md and NBCD.md.
// ---------------------------------------------------------------------------------------------

/// Decimal binary operation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bcd {
    /// `dst + src + X`.
    Abcd,
    /// `dst - src - X`.
    Sbcd,
}

/// Sets the flags of a BCD result: X and C = `carry`, V as computed, N from bit 7, Z only cleared.
fn bcd_flags(sr: u16, res: u32, v: u32, carry: bool) -> u16 {
    let mut sr = sr & !(X | N | V | C);
    if carry {
        sr |= X | C;
    }
    if v & 0x80 != 0 {
        sr |= V;
    }
    if res & 0x80 != 0 {
        sr |= N;
    }
    if res & 0xFF != 0 {
        sr &= !Z;
    }
    sr
}

/// `ABCD` on bytes: result and new status register (behaviour measured on the SST files).
pub fn abcd_calc(src: u32, dst: u32, sr: u16) -> (u32, u16) {
    let x = u32::from(sr & X != 0);
    let (src, dst) = (src & 0xFF, dst & 0xFF);
    let bin = src + dst + x;
    let low = (src & 0xF) + (dst & 0xF) + x;
    let t = if low > 9 { bin + 6 } else { bin };
    let carry = t >= 0xA0;
    let res = (if carry { t + 0x60 } else { t }) & 0xFF;
    (res, bcd_flags(sr, res, !bin & res, carry))
}

/// `SBCD` on bytes: `dst - src - X`.
pub fn sbcd_calc(src: u32, dst: u32, sr: u16) -> (u32, u16) {
    let x = i32::from(sr & X != 0);
    let (src, dst) = ((src & 0xFF) as i32, (dst & 0xFF) as i32);
    let bin = dst - src - x;
    let low = (dst & 0xF) - (src & 0xF) - x;
    let t = if low < 0 { bin - 6 } else { bin };
    let res = (if bin < 0 { t - 0x60 } else { t } as u32) & 0xFF;
    (res, bcd_flags(sr, res, (bin as u32) & !res, t < 0))
}

/// `NBCD`: `0 - dst - X` in decimal.
pub fn nbcd_calc(dst: u32, sr: u16) -> (u32, u16) {
    sbcd_calc(dst, 0, sr)
}

/// Semantics of [`HK_XOP`] for `ABCD` / `SBCD`.
pub fn bcd_exec(op: Bcd, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let dn = ((ird >> 9) & 7) as usize;
    let mem = ird & 8 != 0;
    let src = if mem {
        cpu.lat.aux
    } else {
        cpu.regs.d[(ird & 7) as usize]
    };
    let dst = if mem { cpu.lat.val } else { cpu.regs.d[dn] };
    let (r, sr) = match op {
        Bcd::Abcd => abcd_calc(src, dst, cpu.regs.sr),
        Bcd::Sbcd => sbcd_calc(src, dst, cpu.regs.sr),
    };
    cpu.regs.sr = sr;
    if mem {
        cpu.lat.val = r;
    } else {
        set_dn(cpu, dn, Size::Byte, r);
    }
}

static BCD_REG: [Tok; 3] = [Tok::H(HK_XOP), Tok::Pf, Tok::N(2)];
static NBCD_REG: [Tok; 3] = [Tok::H(HK_UNARY), Tok::Pf, Tok::N(2)];

/// Plan of `ABCD` / `SBCD`.
pub fn bcd_plan(cpu: &Cpu) -> Plan {
    if cpu.lat.ird & 8 == 0 {
        &BCD_REG
    } else {
        x_plan(cpu)
    }
}

/// Hook dispatcher of `ABCD` / `SBCD`.
pub fn bcd_hook(op: Bcd, k: u8, cpu: &mut Cpu) {
    if k == HK_XOP {
        bcd_exec(op, cpu);
    } else {
        x_hook(Alu::Add, k, cpu);
    }
}

/// Plan of `NBCD`.
pub fn nbcd_plan(cpu: &Cpu) -> Plan {
    if (cpu.lat.ird >> 3) & 7 == 0 {
        &NBCD_REG
    } else {
        unary_plan(cpu, false)
    }
}

/// Hook dispatcher of `NBCD`.
pub fn nbcd_hook(k: u8, cpu: &mut Cpu) {
    if k == HK_UNARY {
        let ird = cpu.lat.ird;
        let reg = (ird & 7) as usize;
        if (ird >> 3) & 7 == 0 {
            let (r, sr) = nbcd_calc(cpu.regs.d[reg], cpu.regs.sr);
            set_dn(cpu, reg, Size::Byte, r);
            cpu.regs.sr = sr;
        } else {
            let (r, sr) = nbcd_calc(cpu.lat.val, cpu.regs.sr);
            cpu.lat.val = r;
            cpu.regs.sr = sr;
        }
    } else {
        unary_hook(Unary::Not, k, cpu);
    }
}

/// Number of rules of `NBCD`.
pub const N_RULES_NBCD: usize = IMM_FORMS;

/// Decoding rules of `NBCD` (`0100 1000 00 ea`, data-alterable modes).
pub const fn nbcd_rules(id: InsnId) -> [Rule; N_RULES_NBCD] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_NBCD];
    let mut f = 0;
    while f < IMM_FORMS {
        let r = imm_form_rule(0x4800, 0, f, id);
        out[f] = Rule { variant: 0, ..r };
        f += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Conditions (Bcc, DBcc, Scc).
// ---------------------------------------------------------------------------------------------

/// Evaluates condition `cond` (0 = T, 1 = F, 2 = HI ... 15 = LE) against the status register.
pub fn cond_true(cond: u16, sr: u16) -> bool {
    let c = sr & C != 0;
    let v = sr & V != 0;
    let z = sr & Z != 0;
    let n = sr & N != 0;
    match cond & 15 {
        0 => true,
        1 => false,
        2 => !c && !z,
        3 => c || z,
        4 => !c,
        5 => c,
        6 => !z,
        7 => z,
        8 => !v,
        9 => v,
        10 => !n,
        11 => n,
        12 => n == v,
        13 => n != v,
        14 => !z && n == v,
        _ => z || n != v,
    }
}

// ---------------------------------------------------------------------------------------------
// Scc. Timing: docs/specs/instr/Scc.md.
// ---------------------------------------------------------------------------------------------

static SCC_TRUE_REG: [Tok; 3] = [Tok::H(HK_UNARY), Tok::Pf, Tok::N(2)];

/// Plan of `Scc` (byte read-modify-write; register form: `np`, or `np n` when the condition holds).
pub fn scc_plan(cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    let f = imm_form(ird);
    if f == 0 {
        if cond_true(ird >> 8, cpu.regs.sr) {
            &SCC_TRUE_REG
        } else {
            &SCC_TRUE_REG[..2]
        }
    } else {
        let e = &UNARY_RMW[f];
        &e.toks[..e.n]
    }
}

/// Hook dispatcher of `Scc`.
pub fn scc_hook(k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    if k == HK_UNARY {
        let v = if cond_true(ird >> 8, cpu.regs.sr) {
            0xFF
        } else {
            0
        };
        if (ird >> 3) & 7 == 0 {
            set_dn(cpu, (ird & 7) as usize, Size::Byte, v);
        } else {
            cpu.lat.val = v;
        }
    } else {
        crate::ea::run_ea_hook(cpu, k, (ird & 7) as u8, Size::Byte);
    }
}

/// Number of rules of `Scc`.
pub const N_RULES_SCC: usize = IMM_FORMS;

/// Decoding rules of `Scc` (`0101 cccc 11 ea`, data-alterable modes, any condition).
pub const fn scc_rules(id: InsnId) -> [Rule; N_RULES_SCC] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N_RULES_SCC];
    let mut f = 0;
    while f < IMM_FORMS {
        let r = imm_form_rule(0x50C0, 0, f, id);
        out[f] = Rule {
            mask: 0xF0C0 | (r.mask & 0x3F),
            variant: 0,
            ..r
        };
        f += 1;
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
