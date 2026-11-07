//! Opcode decoding: a 65536-entry table built at compile time.
//!
//! Each instruction module provides rules `(mask, value) -> (id, variant)`.
//! An opcode `o` matches a rule when `o & mask == value`.  Two rules matching the
//! same opcode are an error (compile-time panic); the order of rules is irrelevant.

use crate::insn;

/// Instruction identifier. `InsnId::ILLEGAL` (0) means "no instruction".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InsnId(pub u16);

impl InsnId {
    /// Opcode not recognised.
    pub const ILLEGAL: InsnId = InsnId(0);
}

/// One decoding rule.
#[derive(Clone, Copy, Debug)]
pub struct Rule {
    /// Bits of the opcode that are compared.
    pub mask: u16,
    /// Expected value of the compared bits.
    pub value: u16,
    /// Instruction the rule selects.
    pub id: InsnId,
    /// Instruction-specific variant (size, addressing form, ...).
    pub variant: u8,
}

/// Result of decoding an opcode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Decoded {
    /// Instruction identifier.
    pub id: InsnId,
    /// Variant given by the matching rule.
    pub variant: u8,
}

/// Build the table. Panics (at compile time when used in a `static`) if two rules
/// capture the same opcode.
pub const fn build_table(rules: &[Rule]) -> [Decoded; 65536] {
    let mut t = [Decoded {
        id: InsnId::ILLEGAL,
        variant: 0,
    }; 65536];
    let mut r = 0;
    while r < rules.len() {
        let rule = rules[r];
        let mut o: usize = 0;
        while o < 65536 {
            if (o as u16) & rule.mask == rule.value {
                if t[o].id.0 != 0 {
                    panic!("decode: two rules capture the same opcode");
                }
                t[o] = Decoded {
                    id: rule.id,
                    variant: rule.variant,
                };
            }
            o += 1;
        }
        r += 1;
    }
    t
}

/// The decoding table of the core.
pub static TABLE: [Decoded; 65536] = build_table(insn::RULES);

/// Decode an opcode.
#[inline]
pub fn decode(op: u16) -> Decoded {
    TABLE[op as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rule(mask: u16, value: u16, id: u16, variant: u8) -> Rule {
        Rule {
            mask,
            value,
            id: InsnId(id),
            variant,
        }
    }

    #[test]
    fn e05_01_empty_illegal() {
        let t = build_table(&[]);
        assert!(t.iter().all(|d| d.id == InsnId::ILLEGAL && d.variant == 0));
    }

    #[test]
    fn e05_01_single_rule() {
        let t = build_table(&[rule(0xFFFF, 0x4E71, 7, 0)]);
        assert_eq!(t[0x4E71].id, InsnId(7));
        assert_eq!(t[0x4E72].id, InsnId::ILLEGAL);
    }

    #[test]
    fn e05_01_mask_range() {
        let t = build_table(&[rule(0xF1F8, 0x4000, 3, 0)]);
        assert_eq!(t.iter().filter(|d| d.id == InsnId(3)).count(), 8 * 8);
        assert_eq!(t[0x4000].id, InsnId(3));
        assert_eq!(t[0x4E07].id, InsnId(3));
        assert_eq!(t[0x4008].id, InsnId::ILLEGAL);
    }

    #[test]
    fn e05_01_variant() {
        let t = build_table(&[rule(0xFFF8, 0x4840, 5, 9)]);
        assert_eq!(
            t[0x4843],
            Decoded {
                id: InsnId(5),
                variant: 9
            }
        );
    }

    #[test]
    fn e05_01_table_size() {
        assert_eq!(TABLE.len(), 65536);
        assert_eq!(decode(0x1234).id, InsnId::ILLEGAL);
    }

    #[test]
    #[should_panic(expected = "same opcode")]
    fn e05_01_conflict_detected() {
        let _ = build_table(&[rule(0xFF00, 0x4E00, 1, 0), rule(0xFFFF, 0x4E71, 2, 0)]);
    }
}
