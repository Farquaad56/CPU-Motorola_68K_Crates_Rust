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

pub mod i_abcd;
pub mod i_add;
pub mod i_adda;
pub mod i_addx;
pub mod i_and;
pub mod i_asl;
pub mod i_asr;
pub mod i_bcc;
pub mod i_bchg;
pub mod i_bclr;
pub mod i_bset;
pub mod i_btst;
pub mod i_clr;
pub mod i_cmp;
pub mod i_cmpa;
pub mod i_divs;
pub mod i_divu;
pub mod i_eor;
pub mod i_exg;
pub mod i_ext;
pub mod i_lea;
pub mod i_link;
pub mod i_lsl;
pub mod i_lsr;
pub mod i_move;
pub mod i_movea;
pub mod i_movem;
pub mod i_movep;
pub mod i_moveq;
pub mod i_muls;
pub mod i_mulu;
pub mod i_nbcd;
pub mod i_neg;
pub mod i_negx;
pub mod i_not;
pub mod i_or;
pub mod i_pea;
pub mod i_rol;
pub mod i_ror;
pub mod i_roxl;
pub mod i_roxr;
pub mod i_sbcd;
pub mod i_sub;
pub mod i_suba;
pub mod i_subx;
pub mod i_swap;
pub mod i_tst;
pub mod i_unlink;
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
    i_exg::RULES,
    i_swap::RULES,
    i_ext::RULES,
    i_link::RULES,
    i_unlink::RULES,
    i_clr::RULES,
    i_add::RULES,
    i_sub::RULES,
    i_adda::RULES,
    i_suba::RULES,
    i_addx::RULES,
    i_subx::RULES,
    i_neg::RULES,
    i_negx::RULES,
    i_tst::RULES,
    i_cmp::RULES,
    i_cmpa::RULES,
    i_mulu::RULES,
    i_muls::RULES,
    i_divu::RULES,
    i_divs::RULES,
    i_and::RULES,
    i_or::RULES,
    i_eor::RULES,
    i_not::RULES,
    i_asl::RULES,
    i_asr::RULES,
    i_lsl::RULES,
    i_lsr::RULES,
    i_rol::RULES,
    i_ror::RULES,
    i_roxl::RULES,
    i_roxr::RULES,
    i_btst::RULES,
    i_bset::RULES,
    i_bclr::RULES,
    i_bchg::RULES,
    i_abcd::RULES,
    i_sbcd::RULES,
    i_nbcd::RULES,
    i_bcc::RULES,
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
    i_exg::RULES,
    i_swap::RULES,
    i_ext::RULES,
    i_link::RULES,
    i_unlink::RULES,
    i_clr::RULES,
    i_add::RULES,
    i_sub::RULES,
    i_adda::RULES,
    i_suba::RULES,
    i_addx::RULES,
    i_subx::RULES,
    i_neg::RULES,
    i_negx::RULES,
    i_tst::RULES,
    i_cmp::RULES,
    i_cmpa::RULES,
    i_mulu::RULES,
    i_muls::RULES,
    i_divu::RULES,
    i_divs::RULES,
    i_and::RULES,
    i_or::RULES,
    i_eor::RULES,
    i_not::RULES,
    i_asl::RULES,
    i_asr::RULES,
    i_lsl::RULES,
    i_lsr::RULES,
    i_rol::RULES,
    i_ror::RULES,
    i_roxl::RULES,
    i_roxr::RULES,
    i_btst::RULES,
    i_bset::RULES,
    i_bclr::RULES,
    i_bchg::RULES,
    i_abcd::RULES,
    i_sbcd::RULES,
    i_nbcd::RULES,
    i_bcc::RULES,
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
        i_exg::ID => i_exg::plan(variant),
        i_swap::ID => i_swap::plan(variant),
        i_ext::ID => i_ext::plan(variant),
        i_link::ID => i_link::plan(variant),
        i_unlink::ID => i_unlink::plan(variant),
        i_clr::ID => i_clr::plan(variant, cpu),
        i_add::ID => i_add::plan(variant, cpu),
        i_sub::ID => i_sub::plan(variant, cpu),
        i_adda::ID => i_adda::plan(variant, cpu),
        i_suba::ID => i_suba::plan(variant, cpu),
        i_addx::ID => i_addx::plan(variant, cpu),
        i_subx::ID => i_subx::plan(variant, cpu),
        i_neg::ID => i_neg::plan(variant, cpu),
        i_negx::ID => i_negx::plan(variant, cpu),
        i_tst::ID => i_tst::plan(variant, cpu),
        i_cmp::ID => i_cmp::plan(variant, cpu),
        i_cmpa::ID => i_cmpa::plan(variant, cpu),
        i_mulu::ID => i_mulu::plan(variant, cpu),
        i_muls::ID => i_muls::plan(variant, cpu),
        i_divu::ID => i_divu::plan(variant, cpu),
        i_divs::ID => i_divs::plan(variant, cpu),
        i_and::ID => i_and::plan(variant, cpu),
        i_or::ID => i_or::plan(variant, cpu),
        i_eor::ID => i_eor::plan(variant, cpu),
        i_not::ID => i_not::plan(variant, cpu),
        i_asl::ID => i_asl::plan(variant, cpu),
        i_asr::ID => i_asr::plan(variant, cpu),
        i_lsl::ID => i_lsl::plan(variant, cpu),
        i_lsr::ID => i_lsr::plan(variant, cpu),
        i_rol::ID => i_rol::plan(variant, cpu),
        i_ror::ID => i_ror::plan(variant, cpu),
        i_roxl::ID => i_roxl::plan(variant, cpu),
        i_roxr::ID => i_roxr::plan(variant, cpu),
        i_btst::ID => i_btst::plan(variant, cpu),
        i_bset::ID => i_bset::plan(variant, cpu),
        i_bclr::ID => i_bclr::plan(variant, cpu),
        i_bchg::ID => i_bchg::plan(variant, cpu),
        i_abcd::ID => i_abcd::plan(variant, cpu),
        i_sbcd::ID => i_sbcd::plan(variant, cpu),
        i_nbcd::ID => i_nbcd::plan(variant, cpu),
        i_bcc::ID => i_bcc::plan(variant, cpu),
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
        i_exg::ID => i_exg::hook(k, cpu),
        i_swap::ID => i_swap::hook(k, cpu),
        i_ext::ID => i_ext::hook(k, cpu),
        i_link::ID => i_link::hook(k, cpu),
        i_unlink::ID => i_unlink::hook(k, cpu),
        i_clr::ID => i_clr::hook(k, cpu),
        i_add::ID => i_add::hook(k, cpu),
        i_sub::ID => i_sub::hook(k, cpu),
        i_adda::ID => i_adda::hook(k, cpu),
        i_suba::ID => i_suba::hook(k, cpu),
        i_addx::ID => i_addx::hook(k, cpu),
        i_subx::ID => i_subx::hook(k, cpu),
        i_neg::ID => i_neg::hook(k, cpu),
        i_negx::ID => i_negx::hook(k, cpu),
        i_tst::ID => i_tst::hook(k, cpu),
        i_cmp::ID => i_cmp::hook(k, cpu),
        i_cmpa::ID => i_cmpa::hook(k, cpu),
        i_mulu::ID => i_mulu::hook(k, cpu),
        i_muls::ID => i_muls::hook(k, cpu),
        i_divu::ID => i_divu::hook(k, cpu),
        i_divs::ID => i_divs::hook(k, cpu),
        i_and::ID => i_and::hook(k, cpu),
        i_or::ID => i_or::hook(k, cpu),
        i_eor::ID => i_eor::hook(k, cpu),
        i_not::ID => i_not::hook(k, cpu),
        i_asl::ID => i_asl::hook(k, cpu),
        i_asr::ID => i_asr::hook(k, cpu),
        i_lsl::ID => i_lsl::hook(k, cpu),
        i_lsr::ID => i_lsr::hook(k, cpu),
        i_rol::ID => i_rol::hook(k, cpu),
        i_ror::ID => i_ror::hook(k, cpu),
        i_roxl::ID => i_roxl::hook(k, cpu),
        i_roxr::ID => i_roxr::hook(k, cpu),
        i_btst::ID => i_btst::hook(k, cpu),
        i_bset::ID => i_bset::hook(k, cpu),
        i_bclr::ID => i_bclr::hook(k, cpu),
        i_bchg::ID => i_bchg::hook(k, cpu),
        i_abcd::ID => i_abcd::hook(k, cpu),
        i_sbcd::ID => i_sbcd::hook(k, cpu),
        i_nbcd::ID => i_nbcd::hook(k, cpu),
        i_bcc::ID => i_bcc::hook(k, cpu),
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
