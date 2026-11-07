//! Plan tokens: the vocabulary of micro-cycle plans (draft spec 12).

/// Symbolic address source of a bus token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrSrc {
    /// Program counter.
    Pc,
    /// Effective address computed by the decoder.
    Ea,
    /// Effective address plus 2 (second word of a long access).
    EaPlus2,
    /// Effective address read in program space (PC-relative operands: FC = 2 or 6).
    EaProg,
    /// Address register `An`.
    An(u8),
    /// Pre-decrement of `An`.
    AnPre(u8),
    /// Post-increment of `An`.
    AnPost(u8),
    /// Active stack pointer.
    Sp,
    /// Pre-decrement of the stack pointer.
    SpPre,
    /// Post-increment of the stack pointer.
    SpPost,
    /// Exception vector number.
    Vec(u8),
}

/// One step of a micro-cycle plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    /// Internal cycle of `n` clocks (n even), no bus activity.
    N(u8),
    /// Prefetch: read the word at PC, IRC -> IR, word -> IRC, PC += 2.
    Pf,
    /// Byte read.
    Rb(AddrSrc),
    /// Word read.
    Rw(AddrSrc),
    /// Long read (high word then low word).
    Rl(AddrSrc),
    /// Byte write.
    Wb(AddrSrc),
    /// Word write.
    Ww(AddrSrc),
    /// Long write (two chained bus cycles).
    Wl(AddrSrc),
    /// Indivisible read-modify-write cycle of TAS (duration to confirm, spec 12).
    Tas(AddrSrc),
    /// Calls semantic hook `k` without consuming any clock.
    H(u8),
}

/// A plan is a static list of tokens.
pub type Plan = &'static [Tok];

impl Tok {
    /// Duration of the token in clocks.
    pub const fn clocks(self) -> u32 {
        match self {
            Tok::N(n) => n as u32,
            Tok::Pf | Tok::Rb(_) | Tok::Rw(_) | Tok::Wb(_) | Tok::Ww(_) | Tok::Tas(_) => 4,
            Tok::Rl(_) | Tok::Wl(_) => 8,
            Tok::H(_) => 0,
        }
    }
}

/// Total duration of a plan in clocks.
pub const fn plan_clocks(p: Plan) -> u32 {
    let mut i = 0;
    let mut total = 0;
    while i < p.len() {
        total += p[i].clocks();
        i += 1;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e02_04_clocks_basic() {
        assert_eq!(Tok::N(2).clocks(), 2);
        assert_eq!(Tok::Pf.clocks(), 4);
        assert_eq!(Tok::Rw(AddrSrc::Ea).clocks(), 4);
        assert_eq!(Tok::Rl(AddrSrc::Ea).clocks(), 8);
        assert_eq!(Tok::H(3).clocks(), 0);
        assert_eq!(Tok::Tas(AddrSrc::Ea).clocks(), 4);
        assert_eq!(Tok::Wl(AddrSrc::SpPre).clocks(), 8);
    }

    #[test]
    fn e02_04_plan_sum() {
        const P: Plan = &[Tok::Pf, Tok::N(2), Tok::Rw(AddrSrc::Ea), Tok::Pf];
        assert_eq!(plan_clocks(P), 14);
        assert_eq!(plan_clocks(&[]), 0);
    }

    #[test]
    fn e02_04_const_plan() {
        static P: Plan = &[Tok::Pf, Tok::H(1), Tok::Ww(AddrSrc::An(2)), Tok::Pf];
        const TOTAL: u32 = plan_clocks(P);
        assert_eq!(TOTAL, 12);
        assert_eq!(P.len(), 4);
    }
}
