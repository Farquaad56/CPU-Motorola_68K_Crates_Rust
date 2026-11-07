//! Architectural state of the 68000 (D0-D7, A0-A6, USP, SSP, SR, PC, prefetch queue).

/// SR carry flag.
pub const C: u16 = 0x0001;
/// SR overflow flag.
pub const V: u16 = 0x0002;
/// SR zero flag.
pub const Z: u16 = 0x0004;
/// SR negative flag.
pub const N: u16 = 0x0008;
/// SR extend flag.
pub const X: u16 = 0x0010;
/// SR interrupt mask (bits 8..10).
pub const IMASK: u16 = 0x0700;
/// SR supervisor bit.
pub const S: u16 = 0x2000;
/// SR trace bit.
pub const T: u16 = 0x8000;
/// Mask of the implemented SR bits (T, S, I2-I0, X N Z V C).
pub const SR_MASK: u16 = 0xA71F;

/// CPU registers. A7 is `usp` or `ssp` depending on the S bit of `sr`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    /// Data registers D0-D7.
    pub d: [u32; 8],
    /// Address registers A0-A6 (A7 is `usp` or `ssp`).
    pub a: [u32; 7],
    /// User stack pointer.
    pub usp: u32,
    /// Supervisor stack pointer.
    pub ssp: u32,
    /// Status register.
    pub sr: u16,
    /// Program counter (address of the next word to prefetch, see spec 33).
    pub pc: u32,
    /// Instruction register (first word of the prefetch queue).
    pub ir: u16,
    /// Instruction register capture (second word of the prefetch queue).
    pub irc: u16,
}

impl Regs {
    /// Active stack pointer: SSP if S is set, USP otherwise.
    #[inline]
    pub fn a7(&self) -> u32 {
        if self.sr & S != 0 {
            self.ssp
        } else {
            self.usp
        }
    }

    /// Writes the active stack pointer.
    #[inline]
    pub fn set_a7(&mut self, v: u32) {
        if self.sr & S != 0 {
            self.ssp = v;
        } else {
            self.usp = v;
        }
    }

    /// Address register `n` (0..=7); `n == 7` is the active stack pointer.
    #[inline]
    pub fn an(&self, n: u8) -> u32 {
        let n = n & 7;
        if n == 7 {
            self.a7()
        } else {
            self.a[n as usize]
        }
    }

    /// Writes address register `n` (0..=7); `n == 7` is the active stack pointer.
    #[inline]
    pub fn set_an(&mut self, n: u8, v: u32) {
        let n = n & 7;
        if n == 7 {
            self.set_a7(v);
        } else {
            self.a[n as usize] = v;
        }
    }

    /// Writes SR, keeping only the implemented bits.
    #[inline]
    pub fn set_sr(&mut self, v: u16) {
        self.sr = v & SR_MASK;
    }

    /// Condition code register (low byte of SR).
    #[inline]
    pub fn ccr(&self) -> u8 {
        (self.sr & 0x1F) as u8
    }

    /// Writes the CCR only; the system byte of SR is unchanged.
    #[inline]
    pub fn set_ccr(&mut self, v: u8) {
        self.sr = (self.sr & 0xFF00) | (u16::from(v) & 0x1F);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e02_02_a7_user() {
        let r = Regs {
            usp: 0x1000,
            ssp: 0x2000,
            sr: 0,
            ..Regs::default()
        };
        assert_eq!(r.a7(), 0x1000);
    }

    #[test]
    fn e02_02_a7_super() {
        let r = Regs {
            usp: 0x1000,
            ssp: 0x2000,
            sr: S,
            ..Regs::default()
        };
        assert_eq!(r.a7(), 0x2000);
    }

    #[test]
    fn e02_02_set_a7() {
        let mut r = Regs::default();
        r.set_a7(0x1234);
        assert_eq!((r.usp, r.ssp), (0x1234, 0));
        r.sr = S;
        r.set_a7(0x5678);
        assert_eq!((r.usp, r.ssp), (0x1234, 0x5678));
    }

    #[test]
    fn e02_02_an_7() {
        let mut r = Regs::default();
        r.set_an(3, 0xAAAA);
        assert_eq!(r.an(3), 0xAAAA);
        r.set_an(7, 0x4242);
        assert_eq!(r.an(7), r.a7());
        assert_eq!(r.usp, 0x4242);
        r.sr = S;
        r.ssp = 0x99;
        assert_eq!(r.an(7), 0x99);
        assert_eq!(r.a[6], 0);
    }

    #[test]
    fn e02_02_sr_mask() {
        let mut r = Regs::default();
        r.set_sr(0xFFFF);
        assert_eq!(r.sr, 0xA71F);
    }

    #[test]
    fn e02_02_ccr() {
        let mut r = Regs::default();
        r.set_sr(0xA700);
        r.set_ccr(0xFF);
        assert_eq!(r.sr & 0xFF00, 0xA700);
        assert_eq!(r.ccr(), 0x1F);
        assert_eq!(r.ccr(), (r.sr & 0x1F) as u8);
    }
}
