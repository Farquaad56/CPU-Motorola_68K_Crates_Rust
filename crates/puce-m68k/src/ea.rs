//! Effective address fields and addressing categories (M68000PM chapter 2).
//!
//! An effective address is a 3-bit mode and a 3-bit register field. Categories
//! (data, memory, control, alterable) say which modes an instruction accepts; they
//! are used to write its decoding rules. Modes 7/5..7/7 do not exist.

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

#[cfg(test)]
mod tests {
    use super::*;

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
