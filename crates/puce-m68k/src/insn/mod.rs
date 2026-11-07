//! Instruction modules.
//!
//! Every instruction card adds a submodule `i_xxx` exposing `ID`, `RULES`, `plan` and `hook`, then:
//! 1. declares it below (`pub mod i_xxx;`),
//! 2. adds its `RULES` to `GROUPS` (both the normal and the test list),
//! 3. adds one arm to [`plan_for`] and one to [`run_hook`].
//!
//! The global rule list is the const concatenation of the groups (`concat`); two rules that
//! capture the same opcode make the build fail (see `decode::build_table`).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

pub mod i_lea;
pub mod i_move;
pub mod i_movea;
pub mod i_movem;
pub mod i_movep;
pub mod i_moveq;
pub mod i_pea;
// @mods
pub mod i_nop;

/// Rule groups of the instruction modules.
#[cfg(not(test))]
const GROUPS: &[&[Rule]] = &[
    i_nop::RULES,
    i_move::RULES,
    i_movea::RULES,
    i_moveq::RULES,
    i_movem::RULES,
    i_movep::RULES,
    i_lea::RULES,
    i_pea::RULES,
    // @groups
];

/// Rule groups in unit tests: the instructions plus the test instructions of [`testing`].
#[cfg(test)]
const GROUPS: &[&[Rule]] = &[
    i_nop::RULES,
    i_move::RULES,
    i_movea::RULES,
    i_moveq::RULES,
    i_movem::RULES,
    i_movep::RULES,
    i_lea::RULES,
    i_pea::RULES,
    // @tgroups
    testing::RULES,
];

const fn total(groups: &[&[Rule]]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < groups.len() {
        n += groups[i].len();
        i += 1;
    }
    n
}

const fn concat<const N: usize>(groups: &[&[Rule]]) -> [Rule; N] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; N];
    let mut k = 0;
    let mut i = 0;
    while i < groups.len() {
        let mut j = 0;
        while j < groups[i].len() {
            out[k] = groups[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    out
}

const LEN: usize = total(GROUPS);
static ALL: [Rule; LEN] = concat::<LEN>(GROUPS);

/// All decoding rules of the core.
pub static RULES: &[Rule] = &ALL;

/// Plan of the instruction `(id, variant)` that has just been decoded.
///
/// An identifier without an arm yields an empty plan (the instruction does nothing and takes no
/// clock); `ILLEGAL` never reaches this function (see `Cpu::step`).
#[allow(unused_variables)]
pub(crate) fn plan_for(id: InsnId, variant: u8, cpu: &mut Cpu) -> Plan {
    match id {
        i_nop::ID => i_nop::plan(variant),
        i_move::ID => i_move::plan(variant, cpu),
        i_movea::ID => i_movea::plan(variant, cpu),
        i_moveq::ID => i_moveq::plan(variant),
        i_movem::ID => i_movem::plan(variant, cpu),
        i_movep::ID => i_movep::plan(variant),
        i_lea::ID => i_lea::plan(variant),
        i_pea::ID => i_pea::plan(variant),
        // @plans
        #[cfg(test)]
        testing::ID => testing::plan(variant),
        _ => &[],
    }
}

/// Executes hook `k` of the plan of instruction `id`, between two tokens (no clock is consumed).
#[allow(unused_variables)]
pub(crate) fn run_hook(id: InsnId, k: u8, cpu: &mut Cpu) {
    match id {
        i_nop::ID => i_nop::hook(k, cpu),
        i_move::ID => i_move::hook(k, cpu),
        i_movea::ID => i_movea::hook(k, cpu),
        i_moveq::ID => i_moveq::hook(k, cpu),
        i_movem::ID => i_movem::hook(k, cpu),
        i_movep::ID => i_movep::hook(k, cpu),
        i_lea::ID => i_lea::hook(k, cpu),
        i_pea::ID => i_pea::hook(k, cpu),
        // @hooks
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
