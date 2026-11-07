//! Flat 16 MiB RAM machine (24-bit address bus, 16-bit data bus).

use puce_m68k::bus::Bus;

const SIZE: usize = 1 << 24;
const MASK: u32 = (SIZE as u32) - 1;

/// 16 MiB of RAM with a fast reset: only the words that were written are cleared.
pub struct FlatRam {
    mem: Box<[u8]>,
    /// Even addresses of the words written since the last `clear_touched`.
    touched: Vec<u32>,
}

impl Default for FlatRam {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatRam {
    /// Creates a zeroed 16 MiB RAM.
    pub fn new() -> Self {
        FlatRam {
            mem: vec![0u8; SIZE].into_boxed_slice(),
            touched: Vec::new(),
        }
    }

    /// Stores a 16-bit word at the even address `addr & !1`. The word is remembered so that
    /// `clear_touched` resets it (used to load the initial state of a test).
    pub fn poke_word(&mut self, addr: u32, w: u16) {
        let a = (addr & MASK & !1) as usize;
        self.mem[a] = (w >> 8) as u8;
        self.mem[a + 1] = w as u8;
        self.touched.push(a as u32);
    }

    /// Reads the 16-bit word at `addr & !1` (no side effect).
    pub fn peek_word(&self, addr: u32) -> u16 {
        let a = (addr & MASK & !1) as usize;
        u16::from_be_bytes([self.mem[a], self.mem[a + 1]])
    }

    /// Reads the byte at `addr` (no side effect).
    pub fn peek_byte(&self, addr: u32) -> u8 {
        self.mem[(addr & MASK) as usize]
    }

    /// Zeroes every word written since the last call, then forgets them.
    pub fn clear_touched(&mut self) {
        for &a in &self.touched {
            let a = a as usize;
            self.mem[a] = 0;
            self.mem[a + 1] = 0;
        }
        self.touched.clear();
    }
}

impl Bus for FlatRam {
    fn read(&mut self, addr: u32, _fc: u8, uds: bool, lds: bool) -> u16 {
        let a = (addr & MASK & !1) as usize;
        let hi = if uds { self.mem[a] } else { 0 };
        let lo = if lds { self.mem[a + 1] } else { 0 };
        u16::from_be_bytes([hi, lo])
    }

    fn write(&mut self, addr: u32, _fc: u8, uds: bool, lds: bool, data: u16) {
        let a = (addr & MASK & !1) as usize;
        if uds {
            self.mem[a] = (data >> 8) as u8;
        }
        if lds {
            self.mem[a + 1] = data as u8;
        }
        if uds || lds {
            self.touched.push(a as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e03_02_word_rw() {
        let mut m = FlatRam::new();
        m.write(0x1000, 5, true, true, 0x1234);
        assert_eq!(m.read(0x1000, 5, true, true), 0x1234);
        assert_eq!(m.peek_byte(0x1000), 0x12);
        assert_eq!(m.peek_byte(0x1001), 0x34);
    }

    #[test]
    fn e03_02_byte_high() {
        let mut m = FlatRam::new();
        m.poke_word(0x1000, 0x00CD);
        m.write(0x1000, 5, true, false, 0xAB00);
        assert_eq!(m.peek_byte(0x1000), 0xAB);
        assert_eq!(m.peek_byte(0x1001), 0xCD);
    }

    #[test]
    fn e03_02_byte_low() {
        let mut m = FlatRam::new();
        m.poke_word(0x1000, 0xAB00);
        m.write(0x1001, 5, false, true, 0x00EF);
        assert_eq!(m.peek_byte(0x1000), 0xAB);
        assert_eq!(m.peek_byte(0x1001), 0xEF);
    }

    #[test]
    fn e03_02_read_selects() {
        let mut m = FlatRam::new();
        m.poke_word(0x1000, 0xABCD);
        assert_eq!(m.read(0x1000, 5, true, false), 0xAB00);
        assert_eq!(m.read(0x1000, 5, false, true), 0x00CD);
        assert_eq!(m.read(0x1000, 5, false, false), 0);
    }

    #[test]
    fn e03_02_addr_mask() {
        let mut m = FlatRam::new();
        m.write(0xFF00_1000, 5, true, true, 0xBEEF);
        assert_eq!(m.peek_word(0x00_1000), 0xBEEF);
        assert_eq!(m.read(0x0100_1000, 5, true, true), 0xBEEF);
    }

    #[test]
    fn e03_02_clear_touched() {
        let mut m = FlatRam::new();
        m.write(0x2000, 5, true, true, 0xFFFF);
        m.poke_word(0x3000, 0x1111);
        m.clear_touched();
        assert_eq!(m.peek_word(0x2000), 0);
        assert_eq!(m.peek_word(0x3000), 0);
        assert!(m.touched.is_empty());
    }
}
