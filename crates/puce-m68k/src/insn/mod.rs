//! Instruction modules.
//!
//! Every instruction card adds a submodule exposing `pub const RULES: &[Rule]`; the
//! global list below is their concatenation (add `..` entries through a const
//! concatenation helper when the first instruction module is created).

use crate::decode::Rule;

/// All decoding rules of the core (empty until the first instruction card).
pub const RULES: &[Rule] = &[];
