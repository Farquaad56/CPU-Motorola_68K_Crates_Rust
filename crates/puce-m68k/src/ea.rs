//! Effective address fields and addressing categories (M68000PM chapter 2).
//!
//! An effective address is a 3-bit mode and a 3-bit register field. Categories
//! (data, memory, control, alterable) say which modes an instruction accepts; they
//! are used to write its decoding rules. Modes 7/5..7/7 do not exist.

use crate::cpu::Cpu;
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// The 12 addressing modes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ea {
    /// Data register direct, `Dn`.
    DataReg(u8),
    /// Address register direct, `An`.
    AddrReg(u8),
    /// Address register indirect, `(An)`.
    Ind(u8),
    /// Postincrement, `(An)+`.
    PostInc(u8),
    /// Predecrement, `-(An)`.
    PreDec(u8),
    /// Displacement, `(d16,An)`.
    Disp(u8),
    /// Index, `(d8,An,Xn)`.
    Index(u8),
    /// Absolute short, `(xxx).W`.
    AbsW,
    /// Absolute long, `(xxx).L`.
    AbsL,
    /// PC displacement, `(d16,PC)`.
    PcDisp,
    /// PC index, `(d8,PC,Xn)`.
    PcIndex,
    /// Immediate, `#<data>`.
    Imm,
}

impl Ea {
    /// Decode the mode and register fields. `None` for mode 7 with register 5..7.
    pub const fn decode(mode: u8, reg: u8) -> Option<Ea> {
        let reg = reg & 7;
        match mode & 7 {
            0 => Some(Ea::DataReg(reg)),
            1 => Some(Ea::AddrReg(reg)),
            2 => Some(Ea::Ind(reg)),
            3 => Some(Ea::PostInc(reg)),
            4 => Some(Ea::PreDec(reg)),
            5 => Some(Ea::Disp(reg)),
            6 => Some(Ea::Index(reg)),
            _ => match reg {
                0 => Some(Ea::AbsW),
                1 => Some(Ea::AbsL),
                2 => Some(Ea::PcDisp),
                3 => Some(Ea::PcIndex),
                4 => Some(Ea::Imm),
                _ => None,
            },
        }
    }
}

/// Addressing categories of the manual.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cat {
    /// Everything but `An`.
    Data,
    /// Everything but `Dn` and `An`.
    Memory,
    /// `(An)`, `(d16,An)`, `(d8,An,Xn)`, abs.W, abs.L, `(d16,PC)`, `(d8,PC,Xn)`.
    Control,
    /// Everything but `(d16,PC)`, `(d8,PC,Xn)` and `#imm`.
    Alterable,
    /// Data and alterable.
    DataAlterable,
    /// Memory and alterable.
    MemoryAlterable,
    /// Control and alterable.
    ControlAlterable,
}

/// True when the (mode, reg) pair belongs to the category.
pub const fn in_cat(cat: Cat, mode: u8, reg: u8) -> bool {
    let ea = match Ea::decode(mode, reg) {
        Some(e) => e,
        None => return false,
    };
    let data = !matches!(ea, Ea::AddrReg(_));
    let memory = !matches!(ea, Ea::DataReg(_) | Ea::AddrReg(_));
    let control = matches!(
        ea,
        Ea::Ind(_) | Ea::Disp(_) | Ea::Index(_) | Ea::AbsW | Ea::AbsL | Ea::PcDisp | Ea::PcIndex
    );
    let alterable = !matches!(ea, Ea::PcDisp | Ea::PcIndex | Ea::Imm);
    match cat {
        Cat::Data => data,
        Cat::Memory => memory,
        Cat::Control => control,
        Cat::Alterable => alterable,
        Cat::DataAlterable => data && alterable,
        Cat::MemoryAlterable => memory && alterable,
        Cat::ControlAlterable => control && alterable,
    }
}

/// Data addressing.
pub const fn is_data(mode: u8, reg: u8) -> bool {
    in_cat(Cat::Data, mode, reg)
}
/// Memory addressing.
pub const fn is_memory(mode: u8, reg: u8) -> bool {
    in_cat(Cat::Memory, mode, reg)
}
/// Control addressing.
pub const fn is_control(mode: u8, reg: u8) -> bool {
    in_cat(Cat::Control, mode, reg)
}
/// Alterable addressing.
pub const fn is_alterable(mode: u8, reg: u8) -> bool {
    in_cat(Cat::Alterable, mode, reg)
}
/// Data alterable addressing.
pub const fn is_data_alterable(mode: u8, reg: u8) -> bool {
    in_cat(Cat::DataAlterable, mode, reg)
}
/// Memory alterable addressing.
pub const fn is_memory_alterable(mode: u8, reg: u8) -> bool {
    in_cat(Cat::MemoryAlterable, mode, reg)
}
/// Control alterable addressing.
pub const fn is_control_alterable(mode: u8, reg: u8) -> bool {
    in_cat(Cat::ControlAlterable, mode, reg)
}

/// 64-bit mask of a category, bit `mode*8+reg` set when the pair belongs to it (const).
pub const fn cat_mask(cat: Cat) -> u64 {
    let mut m = 0u64;
    let mut i = 0u8;
    while i < 64 {
        if in_cat(cat, i >> 3, i & 7) {
            m |= 1u64 << i;
        }
        i += 1;
    }
    m
}

/// Same as [`cat_mask`] for any predicate. Not `const`: calling a function pointer is not
/// allowed in `const fn`; use [`cat_mask`] to build rules at compile time.
pub fn ea_mask(f: fn(u8, u8) -> bool) -> u64 {
    let mut m = 0u64;
    for i in 0..64u8 {
        if f(i >> 3, i & 7) {
            m |= 1u64 << i;
        }
    }
    m
}

/// Address source of every operand access (the hooks leave the address in ea).
const A: AddrSrc = AddrSrc::Ea;

/// Operand size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Size {
    /// 8 bits.
    Byte,
    /// 16 bits.
    Word,
    /// 32 bits.
    Long,
}

/// Hook: `ea = An` (register number taken from the instruction by the executor).
pub const HK_EA_AN: u8 = 0x01;
/// Hook: `An -= size` (2 for a byte access on A7), then `ea = An`.
pub const HK_AN_PREDEC: u8 = 0x02;
/// Hook: `An += size` after the access (2 for a byte access on A7).
pub const HK_AN_POSTINC: u8 = 0x03;

/// Hook: `ea = d16 + An` using the extension word (IRC, read before the `Pf` that consumes it).
pub const HK_EA_DISP: u8 = 0x04;
/// Hook: `ea = d8 + An + Xn` using the extension word (the index size does not change the timing).
pub const HK_EA_INDEX: u8 = 0x05;
/// Hook: `ea = sign-extended extension word` (absolute short).
pub const HK_EA_ABSW: u8 = 0x06;
/// Hook: `ea = extension word << 16` (first word of an absolute long).
pub const HK_EA_ABSL_HI: u8 = 0x07;
/// Hook: `ea |= extension word` (second word of an absolute long).
pub const HK_EA_ABSL_LO: u8 = 0x08;
/// Hook: `ea = d16 + PC` using the extension word.
pub const HK_EA_PCDISP: u8 = 0x09;
/// Hook: `ea = d8 + PC + Xn` using the extension word.
pub const HK_EA_PCINDEX: u8 = 0x0A;
/// Hook: immediate operand = extension word (byte or word immediate).
pub const HK_IMM_W: u8 = 0x0B;
/// Hook: immediate operand = extension word << 16 (first word of a long immediate).
pub const HK_IMM_L_HI: u8 = 0x0C;
/// Hook: immediate operand |= extension word (second word of a long immediate).
pub const HK_IMM_L_LO: u8 = 0x0D;

/// Executes an effective-address hook (`HK_EA_*`, `HK_AN_*`, `HK_IMM_*`) of a plan built from
/// [`ea_read_plan`] / [`ea_write_plan`]. `reg` is the register field of the operand (An for the
/// address modes), `size` the operand size. Returns false when `k` is not an EA hook.
///
/// The extension word is `regs.irc`: hooks that read it run before the `Pf` that consumes it.
/// The PC-relative base is the address of the extension word (`PC - 2`, PC being the address of
/// the next word to prefetch).
#[allow(dead_code)] // first user: MOVE (E06-02)
pub(crate) fn run_ea_hook(cpu: &mut Cpu, k: u8, reg: u8, size: Size) -> bool {
    let step = match size {
        Size::Byte if reg & 7 == 7 => 2,
        Size::Byte => 1,
        Size::Word => 2,
        Size::Long => 4,
    };
    let ext = cpu.regs.irc;
    let sext = ext as i16 as i32 as u32;
    match k {
        HK_EA_AN => cpu.lat.ea = cpu.regs.an(reg),
        HK_AN_PREDEC => {
            let v = cpu.regs.an(reg).wrapping_sub(step);
            cpu.regs.set_an(reg, v);
            cpu.lat.ea = v;
        }
        HK_AN_POSTINC => {
            let v = cpu.regs.an(reg).wrapping_add(step);
            cpu.regs.set_an(reg, v);
        }
        HK_EA_DISP => cpu.lat.ea = cpu.regs.an(reg).wrapping_add(sext),
        HK_EA_INDEX => cpu.lat.ea = index_ea(cpu, cpu.regs.an(reg)),
        HK_EA_ABSW => cpu.lat.ea = sext,
        HK_EA_ABSL_HI => cpu.lat.ea = (ext as u32) << 16,
        HK_EA_ABSL_LO => cpu.lat.ea |= ext as u32,
        HK_EA_PCDISP => cpu.lat.ea = cpu.regs.pc.wrapping_sub(2).wrapping_add(sext),
        HK_EA_PCINDEX => cpu.lat.ea = index_ea(cpu, cpu.regs.pc.wrapping_sub(2)),
        HK_IMM_W => {
            cpu.lat.val = match size {
                Size::Byte => (ext & 0xFF) as u32,
                _ => ext as u32,
            }
        }
        HK_IMM_L_HI => cpu.lat.val = (ext as u32) << 16,
        HK_IMM_L_LO => cpu.lat.val |= ext as u32,
        _ => return false,
    }
    true
}

/// `base + d8 + Xn` from the brief extension word in IRC (bit 15: An/Dn, bits 14-12: register,
/// bit 11: long/word index, bits 7-0: displacement).
fn index_ea(cpu: &Cpu, base: u32) -> u32 {
    let ext = cpu.regs.irc;
    let r = ((ext >> 12) & 7) as u8;
    let xn = if ext & 0x8000 != 0 {
        cpu.regs.an(r)
    } else {
        cpu.regs.d[r as usize]
    };
    let xn = if ext & 0x0800 != 0 {
        xn
    } else {
        xn as u16 as i16 as i32 as u32
    };
    let d8 = ext as u8 as i8 as i32 as u32;
    base.wrapping_add(d8).wrapping_add(xn)
}

/// Number of extension words (0, 1 or 2) that follow the opcode for this mode and size.
pub const fn ea_ext_words(ea: Ea, size: Size) -> u8 {
    match ea {
        Ea::DataReg(_) | Ea::AddrReg(_) | Ea::Ind(_) | Ea::PostInc(_) | Ea::PreDec(_) => 0,
        Ea::Disp(_) | Ea::Index(_) | Ea::AbsW | Ea::PcDisp | Ea::PcIndex => 1,
        Ea::AbsL => 2,
        Ea::Imm => match size {
            Size::Long => 2,
            _ => 1,
        },
    }
}

// One match for reads and one for writes: only the bus token differs (`$b`, `$w`, `$l`).
macro_rules! ea_plans {
    ($ea:expr, $size:expr, $b:ident, $w:ident, $l:ident) => {{
        use Tok::*;
        match ($ea, $size) {
            (Ea::Ind(_), Size::Byte) => &[H(HK_EA_AN), $b(A)],
            (Ea::Ind(_), Size::Word) => &[H(HK_EA_AN), $w(A)],
            (Ea::Ind(_), Size::Long) => &[H(HK_EA_AN), $l(A)],
            (Ea::PostInc(_), Size::Byte) => &[H(HK_EA_AN), $b(A), H(HK_AN_POSTINC)],
            (Ea::PostInc(_), Size::Word) => &[H(HK_EA_AN), $w(A), H(HK_AN_POSTINC)],
            (Ea::PostInc(_), Size::Long) => &[H(HK_EA_AN), $l(A), H(HK_AN_POSTINC)],
            (Ea::PreDec(_), Size::Byte) => &[H(HK_AN_PREDEC), N(2), $b(A)],
            (Ea::PreDec(_), Size::Word) => &[H(HK_AN_PREDEC), N(2), $w(A)],
            (Ea::PreDec(_), Size::Long) => &[H(HK_AN_PREDEC), N(2), $l(A)],
            (Ea::Disp(_), Size::Byte) => &[H(HK_EA_DISP), Pf, $b(A)],
            (Ea::Disp(_), Size::Word) => &[H(HK_EA_DISP), Pf, $w(A)],
            (Ea::Disp(_), Size::Long) => &[H(HK_EA_DISP), Pf, $l(A)],
            (Ea::Index(_), Size::Byte) => &[H(HK_EA_INDEX), N(2), Pf, $b(A)],
            (Ea::Index(_), Size::Word) => &[H(HK_EA_INDEX), N(2), Pf, $w(A)],
            (Ea::Index(_), Size::Long) => &[H(HK_EA_INDEX), N(2), Pf, $l(A)],
            (Ea::AbsW, Size::Byte) => &[H(HK_EA_ABSW), Pf, $b(A)],
            (Ea::AbsW, Size::Word) => &[H(HK_EA_ABSW), Pf, $w(A)],
            (Ea::AbsW, Size::Long) => &[H(HK_EA_ABSW), Pf, $l(A)],
            (Ea::AbsL, Size::Byte) => &[H(HK_EA_ABSL_HI), Pf, H(HK_EA_ABSL_LO), Pf, $b(A)],
            (Ea::AbsL, Size::Word) => &[H(HK_EA_ABSL_HI), Pf, H(HK_EA_ABSL_LO), Pf, $w(A)],
            (Ea::AbsL, Size::Long) => &[H(HK_EA_ABSL_HI), Pf, H(HK_EA_ABSL_LO), Pf, $l(A)],
            (Ea::PcDisp, Size::Byte) => &[H(HK_EA_PCDISP), Pf, $b(A)],
            (Ea::PcDisp, Size::Word) => &[H(HK_EA_PCDISP), Pf, $w(A)],
            (Ea::PcDisp, Size::Long) => &[H(HK_EA_PCDISP), Pf, $l(A)],
            (Ea::PcIndex, Size::Byte) => &[H(HK_EA_PCINDEX), N(2), Pf, $b(A)],
            (Ea::PcIndex, Size::Word) => &[H(HK_EA_PCINDEX), N(2), Pf, $w(A)],
            (Ea::PcIndex, Size::Long) => &[H(HK_EA_PCINDEX), N(2), Pf, $l(A)],
            _ => &[],
        }
    }};
}

/// Micro-cycle plan that fetches an operand at `ea`.
///
/// The plans do not depend on the register number: the hooks (executed by the semantic layer,
/// E05-05) read it from the instruction. Timing from `docs/specs/instr/_EA.md` (and, for the
/// PC-relative modes, M68000UM chapter 8: same totals as the `An` forms, see
/// `docs/qa/inbox/E04-03-Q1.md`): `Dn`/`An` take no clock, `(An)` and `(An)+` one bus cycle (two
/// for a long), `-(An)` and the index modes add an internal cycle of 2 clocks, every extension
/// word is a `Pf` (4 clocks). A hook that reads the extension word is placed BEFORE the `Pf` that
/// consumes it (`Cpu::ext_word` is IRC until that `Pf`). The A7 byte rule (step 2 instead of 1)
/// lives in the hooks, not here. `Dn`/`An` return an empty plan.
pub const fn ea_read_plan(ea: Ea, size: Size) -> Plan {
    if let Ea::Imm = ea {
        return match size {
            Size::Long => &[Tok::H(HK_IMM_L_HI), Tok::Pf, Tok::H(HK_IMM_L_LO), Tok::Pf],
            _ => &[Tok::H(HK_IMM_W), Tok::Pf],
        };
    }
    ea_plans!(ea, size, Rb, Rw, Rl)
}

/// Micro-cycle plan that stores an operand at `ea`: same address phase as [`ea_read_plan`],
/// with a write token (`Wb`/`Ww`/`Wl`) in place of the read. Immediate is not alterable: empty.
pub const fn ea_write_plan(ea: Ea, size: Size) -> Plan {
    ea_plans!(ea, size, Wb, Ww, Wl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::plan::plan_clocks;

    #[test]
    fn e05_04a_dn_no_bus() {
        for s in [Size::Byte, Size::Word, Size::Long] {
            assert_eq!(plan_clocks(ea_read_plan(Ea::DataReg(2), s)), 0);
            assert_eq!(plan_clocks(ea_read_plan(Ea::AddrReg(2), s)), 0);
            assert!(ea_read_plan(Ea::DataReg(2), s).is_empty());
        }
    }

    #[test]
    fn e05_04a_ind_word_4() {
        assert_eq!(plan_clocks(ea_read_plan(Ea::Ind(1), Size::Word)), 4);
        assert_eq!(plan_clocks(ea_read_plan(Ea::Ind(1), Size::Byte)), 4);
        assert_eq!(plan_clocks(ea_read_plan(Ea::PostInc(1), Size::Word)), 4);
    }

    #[test]
    fn e05_04a_ind_long_8() {
        assert_eq!(plan_clocks(ea_read_plan(Ea::Ind(1), Size::Long)), 8);
        assert_eq!(plan_clocks(ea_read_plan(Ea::PostInc(1), Size::Long)), 8);
    }

    #[test]
    fn e05_04a_predec_extra() {
        // _EA.md lines 139 and 150: -(An) = 6 clocks (.B/.W), 10 clocks (.L)
        assert_eq!(plan_clocks(ea_read_plan(Ea::PreDec(3), Size::Byte)), 6);
        assert_eq!(plan_clocks(ea_read_plan(Ea::PreDec(3), Size::Word)), 6);
        assert_eq!(plan_clocks(ea_read_plan(Ea::PreDec(3), Size::Long)), 10);
        assert_eq!(plan_clocks(ea_write_plan(Ea::PreDec(3), Size::Word)), 6);
    }

    #[test]
    fn e05_04a_write_same_as_read_bus() {
        let w = ea_write_plan(Ea::Ind(0), Size::Word);
        assert_eq!(w.iter().filter(|t| matches!(t, Tok::Ww(_))).count(), 1);
        assert_eq!(w.iter().filter(|t| matches!(t, Tok::Rw(_))).count(), 0);
        assert_eq!(
            plan_clocks(w),
            plan_clocks(ea_read_plan(Ea::Ind(0), Size::Word))
        );
        assert!(ea_write_plan(Ea::Ind(0), Size::Byte).contains(&Tok::Wb(AddrSrc::Ea)));
        assert!(ea_write_plan(Ea::Ind(0), Size::Long).contains(&Tok::Wl(AddrSrc::Ea)));
    }

    #[test]
    fn e05_04a_postinc_hook() {
        assert!(ea_read_plan(Ea::PostInc(0), Size::Word).contains(&Tok::H(HK_AN_POSTINC)));
        assert!(ea_write_plan(Ea::PostInc(0), Size::Long).contains(&Tok::H(HK_AN_POSTINC)));
        assert!(ea_read_plan(Ea::PreDec(0), Size::Word).contains(&Tok::H(HK_AN_PREDEC)));
        assert!(!ea_read_plan(Ea::Ind(0), Size::Word).contains(&Tok::H(HK_AN_POSTINC)));
    }

    fn count(p: Plan, f: fn(&Tok) -> bool) -> usize {
        p.iter().filter(|t| f(t)).count()
    }
    fn is_pf(t: &Tok) -> bool {
        matches!(t, Tok::Pf)
    }

    #[test]
    fn e05_04b_disp_word_read() {
        let p = ea_read_plan(Ea::Disp(1), Size::Word);
        assert_eq!(count(p, is_pf), 1);
        assert!(p.contains(&Tok::Rw(AddrSrc::Ea)));
        assert_eq!(plan_clocks(p), 8); // _EA.md line 140
    }

    #[test]
    fn e05_04b_index_internal() {
        let p = ea_read_plan(Ea::Index(0), Size::Word);
        assert!(p.contains(&Tok::N(2)));
        assert_eq!(plan_clocks(p), 10); // _EA.md line 141
        assert_eq!(plan_clocks(ea_read_plan(Ea::Index(0), Size::Long)), 14); // line 152
    }

    #[test]
    fn e05_04b_abs_w_long() {
        let p = ea_read_plan(Ea::AbsW, Size::Long);
        assert_eq!(count(p, is_pf), 1);
        assert!(p.contains(&Tok::Rl(AddrSrc::Ea)));
        assert_eq!(plan_clocks(p), 12); // line 153
    }

    #[test]
    fn e05_04b_abs_l_ext_words() {
        assert_eq!(ea_ext_words(Ea::AbsL, Size::Word), 2);
        let p = ea_read_plan(Ea::AbsL, Size::Word);
        assert_eq!(count(p, is_pf), 2);
        assert_eq!(plan_clocks(p), 12); // line 143
        assert_eq!(plan_clocks(ea_read_plan(Ea::AbsL, Size::Long)), 16); // line 154
    }

    #[test]
    fn e05_04b_imm_long_ext_words() {
        assert_eq!(ea_ext_words(Ea::Imm, Size::Long), 2);
        assert_eq!(ea_ext_words(Ea::Imm, Size::Byte), 1);
        assert_eq!(ea_ext_words(Ea::Imm, Size::Word), 1);
        assert_eq!(count(ea_read_plan(Ea::Imm, Size::Long), is_pf), 2);
        assert_eq!(plan_clocks(ea_read_plan(Ea::Imm, Size::Long)), 8); // line 155
        assert_eq!(plan_clocks(ea_read_plan(Ea::Imm, Size::Byte)), 4); // line 144
        assert!(ea_write_plan(Ea::Imm, Size::Word).is_empty());
    }

    #[test]
    fn e05_04b_pc_disp() {
        let p = ea_read_plan(Ea::PcDisp, Size::Word);
        assert_eq!(count(p, is_pf), 1);
        assert!(p.contains(&Tok::Rw(AddrSrc::Ea)));
        assert_eq!(plan_clocks(p), 8); // M68000UM: same as d16(An)
        assert_eq!(plan_clocks(ea_read_plan(Ea::PcIndex, Size::Word)), 10);
    }

    #[test]
    fn e05_04b_ext_words_table() {
        let modes = [
            (Ea::DataReg(0), 0),
            (Ea::AddrReg(0), 0),
            (Ea::Ind(0), 0),
            (Ea::PostInc(0), 0),
            (Ea::PreDec(0), 0),
            (Ea::Disp(0), 1),
            (Ea::Index(0), 1),
            (Ea::AbsW, 1),
            (Ea::AbsL, 2),
            (Ea::PcDisp, 1),
            (Ea::PcIndex, 1),
            (Ea::Imm, 1),
        ];
        for (ea, n) in modes {
            assert_eq!(ea_ext_words(ea, Size::Word), n, "{:?}", ea);
            // the plan fetches exactly that many extension words (Pf)
            assert_eq!(
                count(ea_read_plan(ea, Size::Word), is_pf),
                n as usize,
                "{:?}",
                ea
            );
        }
    }

    #[test]
    fn e05_02_decode_all_12() {
        assert_eq!(Ea::decode(0, 3), Some(Ea::DataReg(3)));
        assert_eq!(Ea::decode(1, 4), Some(Ea::AddrReg(4)));
        assert_eq!(Ea::decode(2, 1), Some(Ea::Ind(1)));
        assert_eq!(Ea::decode(3, 2), Some(Ea::PostInc(2)));
        assert_eq!(Ea::decode(4, 7), Some(Ea::PreDec(7)));
        assert_eq!(Ea::decode(5, 0), Some(Ea::Disp(0)));
        assert_eq!(Ea::decode(6, 6), Some(Ea::Index(6)));
        assert_eq!(Ea::decode(7, 0), Some(Ea::AbsW));
        assert_eq!(Ea::decode(7, 1), Some(Ea::AbsL));
        assert_eq!(Ea::decode(7, 2), Some(Ea::PcDisp));
        assert_eq!(Ea::decode(7, 3), Some(Ea::PcIndex));
        assert_eq!(Ea::decode(7, 4), Some(Ea::Imm));
    }

    #[test]
    fn e05_02_decode_invalid() {
        for r in 5..8 {
            assert_eq!(Ea::decode(7, r), None);
        }
    }

    #[test]
    fn e05_02_data_excludes_an() {
        for n in 0..8 {
            assert!(!is_data(1, n));
        }
        assert!(is_data(0, 0) && is_data(7, 4));
    }

    #[test]
    fn e05_02_memory_excludes_dn_an() {
        for n in 0..8 {
            assert!(!is_memory(0, n));
            assert!(!is_memory(1, n));
        }
        assert!(is_memory(2, 0) && is_memory(7, 4));
    }

    #[test]
    fn e05_02_control_list() {
        let mut modes = [false; 64];
        for m in 0..8u8 {
            for r in 0..8u8 {
                if is_control(m, r) {
                    modes[(m * 8 + r) as usize] = true;
                }
            }
        }
        // 7 mode kinds: (An), d16(An), d8(An,Xn), abs.W, abs.L, d16(PC), d8(PC,Xn)
        let mut kinds = 0;
        for (m, r) in [(2u8, 0u8), (5, 0), (6, 0), (7, 0), (7, 1), (7, 2), (7, 3)] {
            assert!(is_control(m, r));
            kinds += 1;
        }
        assert_eq!(kinds, 7);
        // 3 modes x 8 registers + 4 mode-7 forms
        assert_eq!(cat_mask(Cat::Control).count_ones(), 3 * 8 + 4);
        assert_eq!(modes.iter().filter(|b| **b).count(), 28);
        assert!(!is_control(3, 0) && !is_control(4, 0) && !is_control(7, 4));
    }

    #[test]
    fn e05_02_alterable_excludes_pc_imm() {
        assert!(!is_alterable(7, 2));
        assert!(!is_alterable(7, 3));
        assert!(!is_alterable(7, 4));
        assert!(is_alterable(1, 0) && is_alterable(7, 1));
    }

    #[test]
    fn e05_02_data_alterable_count() {
        assert_eq!(cat_mask(Cat::DataAlterable).count_ones(), 50);
        assert_eq!(cat_mask(Cat::MemoryAlterable).count_ones(), 42);
        assert_eq!(cat_mask(Cat::ControlAlterable).count_ones(), 26);
    }

    #[test]
    fn e05_02_mask_bit() {
        let m = ea_mask(is_data);
        assert_ne!(m & (1 << 3), 0);
        assert_eq!(m & (1 << (8 + 3)), 0);
        assert_eq!(m, cat_mask(Cat::Data));
    }
}
