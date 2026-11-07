//! Instruction modules.
//!
//! Every instruction card adds a submodule exposing `pub const RULES: &[Rule]`; the
//! global list below is their concatenation (add `..` entries through a const
//! concatenation helper when the first instruction module is created).
//! The same cards add their arm to [`plan_for`] (plan of an instruction) and to
//! [`run_hook`] (semantics of the hooks `H(k)` of their plans).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// All decoding rules of the core (empty until the first instruction card).
#[cfg(not(test))]
pub const RULES: &[Rule] = &[];

/// Decoding rules of the unit tests: the test instructions of [`testing`].
#[cfg(test)]
pub const RULES: &[Rule] = testing::RULES;

/// Plan of the instruction `(id, variant)` that has just been decoded.
///
/// An identifier without an arm yields an empty plan (the instruction does nothing and takes no
/// clock); `ILLEGAL` never reaches this function (see `Cpu::step`).
#[allow(unused_variables)]
pub(crate) fn plan_for(id: InsnId, variant: u8, cpu: &mut Cpu) -> Plan {
    match id {
        #[cfg(test)]
        testing::ID => testing::plan(variant),
        _ => &[],
    }
}

/// Executes hook `k` of the plan of instruction `id`, between two tokens (no clock is consumed).
#[allow(unused_variables)]
pub(crate) fn run_hook(id: InsnId, k: u8, cpu: &mut Cpu) {
    match id {
        #[cfg(test)]
        testing::ID => testing::hook(k, cpu),
        _ => {}
    }
}

/// Test instructions (unit tests only): opcode `0x7FFF` (variant 0) and `0x7FFE` (variant 1),
/// encodings that no real instruction uses (MOVEQ has bit 8 clear).
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::engine::plan::Tok;

    pub const ID: InsnId = InsnId(0xFFFF);
    pub const OP_HOOKS: u16 = 0x7FFF;
    pub const OP_CLOCKS: u16 = 0x7FFE;

    pub const RULES: &[Rule] = &[
        Rule {
            mask: 0xFFFF,
            value: OP_HOOKS,
            id: ID,
            variant: 0,
        },
        Rule {
            mask: 0xFFFF,
            value: OP_CLOCKS,
            id: ID,
            variant: 1,
        },
    ];

    pub fn plan(variant: u8) -> Plan {
        match variant {
            0 => &[Tok::H(1), Tok::Pf, Tok::H(2)],
            _ => &[Tok::N(2), Tok::Pf, Tok::Pf],
        }
    }

    /// Hook 1: counts calls in D0 and records PC in D2. Hook 2: records PC in D1.
    pub fn hook(k: u8, cpu: &mut Cpu) {
        match k {
            1 => {
                cpu.regs.d[0] += 1;
                cpu.regs.d[2] = cpu.regs.pc;
            }
            _ => cpu.regs.d[1] = cpu.regs.pc,
        }
    }
}
