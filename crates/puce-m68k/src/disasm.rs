//! Motorola 68000 disassembler (feature `disasm`), without allocation.
//!
//! `disasm(words, pc, out)` writes one instruction in Motorola syntax (`move.w d0,(a1)+`) and
//! returns the number of words consumed. Opcodes the core does not decode are written `dc.w $xxxx`
//! (length 1), except the valid instructions whose execution is deferred (TRAP, TRAPV, CHK, RTE,
//! STOP, RESET, ILLEGAL: D-043), which are still named. Numbers are lowercase hexadecimal without
//! padding (`$1234.w`, `#$12`); counts, quick values and TRAP numbers are decimal; branch targets
//! are absolute addresses computed from `pc` (address of the opcode word); A7 is written `sp`
//! in operands.

use crate::decode::{decode, InsnId};
use crate::ea::Ea;
use core::fmt::{self, Write};

const MNEMONICS: [&str; 69] = [
    "dc.w", "nop", "move", "movea", "moveq", "movem", "movep", "lea", "pea", "exg", "swap", "ext",
    "link", "unlk", "clr", "add", "sub", "adda", "suba", "addx", "subx", "neg", "negx", "tst",
    "cmp", "cmpa", "mulu", "muls", "divu", "divs", "and", "or", "eor", "not", "asl", "asr", "lsl",
    "lsr", "rol", "ror", "roxl", "roxr", "btst", "bset", "bclr", "bchg", "abcd", "sbcd", "nbcd",
    "b", "bsr", "db", "s", "jmp", "jsr", "rts", "rtr", "andi", "andi", "ori", "ori", "eori",
    "eori", "move", "move", "move", "move", "move", "tas",
];

/// Leading mnemonic of an instruction identifier (`dc.w` for `InsnId::ILLEGAL`). The text written
/// by [`disasm`] for an opcode of that instruction starts with it.
pub fn mnemonic(id: InsnId) -> &'static str {
    MNEMONICS.get(usize::from(id.0)).copied().unwrap_or("dc.w")
}

const CC: [&str; 16] = [
    "t", "f", "hi", "ls", "cc", "cs", "ne", "eq", "vc", "vs", "pl", "mi", "ge", "lt", "gt", "le",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sz {
    B,
    W,
    L,
}

impl Sz {
    fn from_bits(b: u16) -> Sz {
        match b & 3 {
            0 => Sz::B,
            1 => Sz::W,
            _ => Sz::L,
        }
    }

    fn sfx(self) -> &'static str {
        match self {
            Sz::B => "b",
            Sz::W => "w",
            Sz::L => "l",
        }
    }
}

/// Valid instructions of the 68000 that the core does not decode yet (execution deferred).
fn deferred(op: u16) -> bool {
    if matches!(
        op,
        0x4AFC | 0x4E70 | 0x4E72 | 0x4E73 | 0x4E76 | 0x4E40..=0x4E4F
    ) {
        return true;
    }
    if op & 0xF1C0 == 0x4180 {
        let mode = ((op >> 3) & 7) as u8;
        return mode != 1 && Ea::decode(mode, (op & 7) as u8).is_some();
    }
    false
}

fn an(o: &mut dyn Write, n: u8) -> fmt::Result {
    if n == 7 {
        o.write_str("sp")
    } else {
        write!(o, "a{n}")
    }
}

fn disp(o: &mut dyn Write, v: i32) -> fmt::Result {
    if v < 0 {
        write!(o, "-${:x}", -v)
    } else {
        write!(o, "${v:x}")
    }
}

fn reg_name(o: &mut dyn Write, i: u32) -> fmt::Result {
    if i < 8 {
        write!(o, "d{i}")
    } else {
        write!(o, "a{}", i - 8)
    }
}

/// Register list of MOVEM (bit 0 = d0 ... bit 15 = a7): `d0-d3/a0-a3`.
fn list(o: &mut dyn Write, m: u16) -> fmt::Result {
    let mut first = true;
    let mut i = 0u32;
    while i < 16 {
        if m & (1 << i) == 0 {
            i += 1;
            continue;
        }
        let bank_end = (i / 8 + 1) * 8;
        let mut j = i;
        while j + 1 < bank_end && m & (1 << (j + 1)) != 0 {
            j += 1;
        }
        if !first {
            o.write_char('/')?;
        }
        first = false;
        reg_name(o, i)?;
        if j > i {
            o.write_char('-')?;
            reg_name(o, j)?;
        }
        i = j + 1;
    }
    Ok(())
}

struct Dis<'a> {
    w: &'a [u16],
    n: usize,
    pc: u32,
}

impl Dis<'_> {
    fn ext(&mut self) -> u16 {
        let v = self.w.get(self.n).copied().unwrap_or(0);
        self.n += 1;
        v
    }

    fn index(&mut self, o: &mut dyn Write, x: u16) -> fmt::Result {
        if x & 0x8000 != 0 {
            an(o, ((x >> 12) & 7) as u8)?;
        } else {
            write!(o, "d{}", (x >> 12) & 7)?;
        }
        o.write_str(if x & 0x0800 != 0 { ".l" } else { ".w" })
    }

    fn imm(&mut self, o: &mut dyn Write, sz: Sz) -> fmt::Result {
        match sz {
            Sz::B => {
                let v = self.ext() & 0xFF;
                write!(o, "#${v:x}")
            }
            Sz::W => {
                let v = self.ext();
                write!(o, "#${v:x}")
            }
            Sz::L => {
                let hi = u32::from(self.ext());
                let lo = u32::from(self.ext());
                write!(o, "#${:x}", (hi << 16) | lo)
            }
        }
    }

    fn ea(&mut self, o: &mut dyn Write, mode: u8, reg: u8, sz: Sz) -> fmt::Result {
        match mode {
            0 => write!(o, "d{reg}"),
            1 => an(o, reg),
            2 => {
                o.write_char('(')?;
                an(o, reg)?;
                o.write_char(')')
            }
            3 => {
                o.write_char('(')?;
                an(o, reg)?;
                o.write_str(")+")
            }
            4 => {
                o.write_str("-(")?;
                an(o, reg)?;
                o.write_char(')')
            }
            5 => {
                let d = self.ext() as i16;
                disp(o, i32::from(d))?;
                o.write_char('(')?;
                an(o, reg)?;
                o.write_char(')')
            }
            6 => {
                let x = self.ext();
                disp(o, i32::from(x as i8))?;
                o.write_char('(')?;
                an(o, reg)?;
                o.write_char(',')?;
                self.index(o, x)?;
                o.write_char(')')
            }
            _ => match reg {
                0 => {
                    let v = self.ext();
                    write!(o, "${v:x}.w")
                }
                1 => {
                    let hi = u32::from(self.ext());
                    let lo = u32::from(self.ext());
                    write!(o, "${:x}.l", (hi << 16) | lo)
                }
                2 => {
                    let d = self.ext() as i16;
                    disp(o, i32::from(d))?;
                    o.write_str("(pc)")
                }
                3 => {
                    let x = self.ext();
                    disp(o, i32::from(x as i8))?;
                    o.write_str("(pc,")?;
                    self.index(o, x)?;
                    o.write_char(')')
                }
                4 => self.imm(o, sz),
                _ => o.write_char('?'),
            },
        }
    }

    fn target(&self, d: i32) -> u32 {
        self.pc.wrapping_add(2).wrapping_add(d as u32)
    }

    /// `name.sz <ea>` on the ea fields of `op`.
    fn one(&mut self, o: &mut dyn Write, name: &str, op: u16, sz: Sz) -> Result<bool, fmt::Error> {
        write!(o, "{name}.{} ", sz.sfx())?;
        self.ea(o, ((op >> 3) & 7) as u8, (op & 7) as u8, sz)?;
        Ok(true)
    }

    /// `name <ea>` without size.
    fn plain(
        &mut self,
        o: &mut dyn Write,
        name: &str,
        op: u16,
        sz: Sz,
    ) -> Result<bool, fmt::Error> {
        write!(o, "{name} ")?;
        self.ea(o, ((op >> 3) & 7) as u8, (op & 7) as u8, sz)?;
        Ok(true)
    }

    fn g0(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let dn = (op >> 9) & 7;
        if op & 0x0100 != 0 {
            if mode == 1 {
                let opm = (op >> 6) & 7;
                if opm < 4 {
                    return Ok(false);
                }
                let sz = if opm & 1 == 0 { "w" } else { "l" };
                let d = i32::from(self.ext() as i16);
                if opm < 6 {
                    write!(o, "movep.{sz} ")?;
                    disp(o, d)?;
                    o.write_char('(')?;
                    an(o, reg)?;
                    write!(o, "),d{dn}")?;
                } else {
                    write!(o, "movep.{sz} d{dn},")?;
                    disp(o, d)?;
                    o.write_char('(')?;
                    an(o, reg)?;
                    o.write_char(')')?;
                }
                return Ok(true);
            }
            let name = ["btst", "bchg", "bclr", "bset"][usize::from((op >> 6) & 3)];
            write!(o, "{name} d{dn},")?;
            self.ea(o, mode, reg, Sz::B)?;
            return Ok(true);
        }
        let k = (op >> 9) & 7;
        if k == 4 {
            let name = ["btst", "bchg", "bclr", "bset"][usize::from((op >> 6) & 3)];
            let n = self.ext() & 0xFF;
            write!(o, "{name} #{n},")?;
            self.ea(o, mode, reg, Sz::B)?;
            return Ok(true);
        }
        let name = match k {
            0 => "ori",
            1 => "andi",
            2 => "subi",
            3 => "addi",
            5 => "eori",
            6 => "cmpi",
            _ => return Ok(false),
        };
        let szb = (op >> 6) & 3;
        if szb == 3 {
            return Ok(false);
        }
        let sz = Sz::from_bits(szb);
        if mode == 7 && reg == 4 && matches!(k, 0 | 1 | 5) {
            if szb == 0 {
                let v = self.ext() & 0xFF;
                write!(o, "{name} #${v:x},ccr")?;
            } else if szb == 1 {
                let v = self.ext();
                write!(o, "{name} #${v:x},sr")?;
            } else {
                return Ok(false);
            }
            return Ok(true);
        }
        write!(o, "{name}.{} ", sz.sfx())?;
        self.imm(o, sz)?;
        o.write_char(',')?;
        self.ea(o, mode, reg, sz)?;
        Ok(true)
    }

    fn mv(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let sz = match op >> 12 {
            1 => Sz::B,
            3 => Sz::W,
            _ => Sz::L,
        };
        let dmode = ((op >> 6) & 7) as u8;
        let dreg = ((op >> 9) & 7) as u8;
        write!(
            o,
            "{}.{} ",
            if dmode == 1 { "movea" } else { "move" },
            sz.sfx()
        )?;
        self.ea(o, ((op >> 3) & 7) as u8, (op & 7) as u8, sz)?;
        o.write_char(',')?;
        self.ea(o, dmode, dreg, sz)?;
        Ok(true)
    }

    fn movem(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let mask = self.ext();
        let sz = if op & 0x40 != 0 { Sz::L } else { Sz::W };
        write!(o, "movem.{} ", sz.sfx())?;
        if op & 0x0400 != 0 {
            self.ea(o, mode, reg, sz)?;
            o.write_char(',')?;
            list(o, mask)?;
        } else {
            list(o, if mode == 4 { mask.reverse_bits() } else { mask })?;
            o.write_char(',')?;
            self.ea(o, mode, reg, sz)?;
        }
        Ok(true)
    }

    fn g4(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let dn = ((op >> 9) & 7) as u8;
        match op {
            0x4AFC => return o.write_str("illegal").map(|()| true),
            0x4E70 => return o.write_str("reset").map(|()| true),
            0x4E71 => return o.write_str("nop").map(|()| true),
            0x4E72 => {
                let v = self.ext();
                return write!(o, "stop #${v:x}").map(|()| true);
            }
            0x4E73 => return o.write_str("rte").map(|()| true),
            0x4E75 => return o.write_str("rts").map(|()| true),
            0x4E76 => return o.write_str("trapv").map(|()| true),
            0x4E77 => return o.write_str("rtr").map(|()| true),
            0x4E40..=0x4E4F => return write!(o, "trap #{}", op & 15).map(|()| true),
            0x4E50..=0x4E57 => {
                let d = self.ext() as i16;
                o.write_str("link ")?;
                an(o, reg)?;
                return write!(o, ",#{d}").map(|()| true);
            }
            0x4E58..=0x4E5F => {
                o.write_str("unlk ")?;
                an(o, reg)?;
                return Ok(true);
            }
            0x4E60..=0x4E67 => {
                o.write_str("move ")?;
                an(o, reg)?;
                return o.write_str(",usp").map(|()| true);
            }
            0x4E68..=0x4E6F => {
                o.write_str("move usp,")?;
                an(o, reg)?;
                return Ok(true);
            }
            _ => {}
        }
        match op & 0xFFC0 {
            0x4E80 => return self.plain(o, "jsr", op, Sz::L),
            0x4EC0 => return self.plain(o, "jmp", op, Sz::L),
            0x40C0 => {
                o.write_str("move sr,")?;
                self.ea(o, mode, reg, Sz::W)?;
                return Ok(true);
            }
            0x44C0 => {
                o.write_str("move ")?;
                self.ea(o, mode, reg, Sz::W)?;
                return o.write_str(",ccr").map(|()| true);
            }
            0x46C0 => {
                o.write_str("move ")?;
                self.ea(o, mode, reg, Sz::W)?;
                return o.write_str(",sr").map(|()| true);
            }
            0x4AC0 => return self.plain(o, "tas", op, Sz::B),
            0x4800 => return self.plain(o, "nbcd", op, Sz::B),
            0x4840 => {
                if mode == 0 {
                    return write!(o, "swap d{reg}").map(|()| true);
                }
                return self.plain(o, "pea", op, Sz::L);
            }
            0x4880 | 0x48C0 if mode == 0 => {
                let s = if op & 0x40 != 0 { "l" } else { "w" };
                return write!(o, "ext.{s} d{reg}").map(|()| true);
            }
            _ => {}
        }
        if op & 0xFB80 == 0x4880 {
            return self.movem(op, o);
        }
        match op & 0xF1C0 {
            0x41C0 => {
                self.plain(o, "lea", op, Sz::L)?;
                o.write_char(',')?;
                an(o, dn)?;
                return Ok(true);
            }
            0x4180 => {
                self.one(o, "chk", op, Sz::W)?;
                return write!(o, ",d{dn}").map(|()| true);
            }
            _ => {}
        }
        let name = match op & 0xFF00 {
            0x4000 => "negx",
            0x4200 => "clr",
            0x4400 => "neg",
            0x4600 => "not",
            0x4A00 => "tst",
            _ => return Ok(false),
        };
        if (op >> 6) & 3 == 3 {
            return Ok(false);
        }
        self.one(o, name, op, Sz::from_bits(op >> 6))
    }

    fn g5(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let cc = CC[usize::from((op >> 8) & 15)];
        if (op >> 6) & 3 == 3 {
            if mode == 1 {
                let d = i32::from(self.ext() as i16);
                let t = self.target(d);
                return write!(o, "db{cc} d{reg},${t:x}").map(|()| true);
            }
            write!(o, "s{cc} ")?;
            self.ea(o, mode, reg, Sz::B)?;
            return Ok(true);
        }
        let name = if op & 0x0100 != 0 { "subq" } else { "addq" };
        let q = match (op >> 9) & 7 {
            0 => 8,
            q => q,
        };
        let sz = Sz::from_bits(op >> 6);
        write!(o, "{name}.{} #{q},", sz.sfx())?;
        self.ea(o, mode, reg, sz)?;
        Ok(true)
    }

    fn g6(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let c = usize::from((op >> 8) & 15);
        let (name, rest) = match c {
            0 => ("bra", ""),
            1 => ("bsr", ""),
            _ => ("b", CC[c]),
        };
        let d8 = op & 0xFF;
        if d8 == 0 {
            let d = i32::from(self.ext() as i16);
            let t = self.target(d);
            write!(o, "{name}{rest}.w ${t:x}")?;
        } else {
            let t = self.target(i32::from(d8 as u8 as i8));
            write!(o, "{name}{rest}.s ${t:x}")?;
        }
        Ok(true)
    }

    /// Groups 8 (OR), 9 (SUB), B (CMP/EOR), C (AND), D (ADD) share the `ea,Dn` / `Dn,ea` shape.
    fn arith(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let g = op >> 12;
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let dn = ((op >> 9) & 7) as u8;
        let opm = (op >> 6) & 7;
        let sz = Sz::from_bits(opm);
        // Address-register destination (SUBA, CMPA, ADDA) and 16-bit multiply/divide.
        if opm == 3 || opm == 7 {
            match g {
                0x8 | 0xC => {
                    let name = match (g, opm) {
                        (0x8, 3) => "divu.w",
                        (0x8, _) => "divs.w",
                        (_, 3) => "mulu.w",
                        _ => "muls.w",
                    };
                    write!(o, "{name} ")?;
                    self.ea(o, mode, reg, Sz::W)?;
                    return write!(o, ",d{dn}").map(|()| true);
                }
                _ => {
                    let name = match g {
                        0x9 => "suba",
                        0xB => "cmpa",
                        _ => "adda",
                    };
                    let s = if opm == 3 { Sz::W } else { Sz::L };
                    write!(o, "{name}.{} ", s.sfx())?;
                    self.ea(o, mode, reg, s)?;
                    o.write_char(',')?;
                    an(o, dn)?;
                    return Ok(true);
                }
            }
        }
        if opm < 3 {
            let name = match g {
                0x8 => "or",
                0x9 => "sub",
                0xB => "cmp",
                0xC => "and",
                _ => "add",
            };
            write!(o, "{name}.{} ", sz.sfx())?;
            self.ea(o, mode, reg, sz)?;
            return write!(o, ",d{dn}").map(|()| true);
        }
        // opmode 4..6: Dn,ea or one of the register-to-register forms.
        let sz = Sz::from_bits(opm & 3);
        match g {
            0x8 if mode < 2 && opm == 4 => {
                o.write_str("sbcd ")?;
                self.pair(o, mode, reg, dn)?;
                Ok(true)
            }
            0xC if mode < 2 && opm == 4 => {
                o.write_str("abcd ")?;
                self.pair(o, mode, reg, dn)?;
                Ok(true)
            }
            0x9 | 0xD if mode < 2 => {
                let name = if g == 0x9 { "subx" } else { "addx" };
                write!(o, "{name}.{} ", sz.sfx())?;
                self.pair(o, mode, reg, dn)?;
                Ok(true)
            }
            0xB if mode == 1 => {
                write!(o, "cmpm.{} (", sz.sfx())?;
                an(o, reg)?;
                o.write_str(")+,(")?;
                an(o, dn)?;
                o.write_str(")+")?;
                Ok(true)
            }
            0xC if opm == 5 && mode == 0 => write!(o, "exg d{dn},d{reg}").map(|()| true),
            0xC if opm == 5 && mode == 1 => {
                o.write_str("exg ")?;
                an(o, dn)?;
                o.write_char(',')?;
                an(o, reg)?;
                Ok(true)
            }
            0xC if opm == 6 && mode == 1 => {
                write!(o, "exg d{dn},")?;
                an(o, reg)?;
                Ok(true)
            }
            _ => {
                let name = match g {
                    0x8 => "or",
                    0x9 => "sub",
                    0xB => "eor",
                    0xC => "and",
                    _ => "add",
                };
                write!(o, "{name}.{} d{dn},", sz.sfx())?;
                self.ea(o, mode, reg, sz)?;
                Ok(true)
            }
        }
    }

    /// `dY,dX` (mode 0) or `-(aY),-(aX)` (mode 1) of ABCD/SBCD/ADDX/SUBX.
    fn pair(&mut self, o: &mut dyn Write, mode: u8, y: u8, x: u8) -> fmt::Result {
        if mode == 0 {
            write!(o, "d{y},d{x}")
        } else {
            o.write_str("-(")?;
            an(o, y)?;
            o.write_str("),-(")?;
            an(o, x)?;
            o.write_char(')')
        }
    }

    fn shift(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        let mode = ((op >> 3) & 7) as u8;
        let reg = (op & 7) as u8;
        let dir = if op & 0x0100 != 0 { 'l' } else { 'r' };
        let names = ["as", "ls", "rox", "ro"];
        if (op >> 6) & 3 == 3 {
            let name = names[usize::from((op >> 9) & 3)];
            write!(o, "{name}{dir}.w ")?;
            self.ea(o, mode, reg, Sz::W)?;
            return Ok(true);
        }
        let name = names[usize::from((op >> 3) & 3)];
        let sz = Sz::from_bits(op >> 6);
        let cnt = (op >> 9) & 7;
        write!(o, "{name}{dir}.{} ", sz.sfx())?;
        if op & 0x20 != 0 {
            write!(o, "d{cnt}")?;
        } else {
            write!(o, "#{}", if cnt == 0 { 8 } else { cnt })?;
        }
        write!(o, ",d{reg}")?;
        Ok(true)
    }

    fn insn(&mut self, op: u16, o: &mut dyn Write) -> Result<bool, fmt::Error> {
        match op >> 12 {
            0x0 => self.g0(op, o),
            0x1..=0x3 => self.mv(op, o),
            0x4 => self.g4(op, o),
            0x5 => self.g5(op, o),
            0x6 => self.g6(op, o),
            0x7 => {
                if op & 0x0100 != 0 {
                    return Ok(false);
                }
                write!(o, "moveq #{},d{}", i32::from(op as u8 as i8), (op >> 9) & 7).map(|()| true)
            }
            0x8 | 0x9 | 0xB | 0xC | 0xD => self.arith(op, o),
            0xE => self.shift(op, o),
            _ => Ok(false),
        }
    }
}

/// Disassembles the instruction at `words[0]` (located at address `pc`) into `out` and returns the
/// number of words it takes. An opcode that is not an instruction gives `dc.w $xxxx` and 1.
pub fn disasm(words: &[u16], pc: u32, out: &mut dyn Write) -> usize {
    let op = words.first().copied().unwrap_or(0);
    let mut d = Dis { w: words, n: 1, pc };
    let known = decode(op).id != InsnId::ILLEGAL || deferred(op);
    if known && matches!(d.insn(op, out), Ok(true)) {
        return d.n;
    }
    let _ = write!(out, "dc.w ${op:04x}");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Buf {
        b: [u8; 96],
        n: usize,
    }

    impl Write for Buf {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let by = s.as_bytes();
            self.b[self.n..self.n + by.len()].copy_from_slice(by);
            self.n += by.len();
            Ok(())
        }
    }

    impl Buf {
        fn s(&self) -> &str {
            core::str::from_utf8(&self.b[..self.n]).unwrap()
        }
    }

    fn dis(words: &[u16], pc: u32) -> (Buf, usize) {
        let mut b = Buf { b: [0; 96], n: 0 };
        let n = disasm(words, pc, &mut b);
        (b, n)
    }

    fn check(words: &[u16], pc: u32, text: &str) {
        let (b, n) = dis(words, pc);
        assert_eq!(b.s(), text);
        assert_eq!(n, words.len(), "{text}");
    }

    #[test]
    fn e12_01_ea_all_modes() {
        check(&[0x3000], 0, "move.w d0,d0");
        check(&[0x3009], 0, "move.w a1,d0");
        check(&[0x3012], 0, "move.w (a2),d0");
        check(&[0x301A], 0, "move.w (a2)+,d0");
        check(&[0x3022], 0, "move.w -(a2),d0");
        check(&[0x302A, 0x0010], 0, "move.w $10(a2),d0");
        check(&[0x302A, 0xFFF0], 0, "move.w -$10(a2),d0");
        check(&[0x3032, 0x3010], 0, "move.w $10(a2,d3.w),d0");
        check(&[0x3038, 0x1234], 0, "move.w $1234.w,d0");
        check(&[0x3039, 0x0012, 0x3456], 0, "move.w $123456.l,d0");
        check(&[0x303A, 0x0010], 0, "move.w $10(pc),d0");
        check(&[0x303B, 0x3810], 0, "move.w $10(pc,d3.l),d0");
        check(&[0x303C, 0x0012], 0, "move.w #$12,d0");
        check(&[0x3F00], 0, "move.w d0,-(sp)");
        check(&[0x3032, 0xF810], 0, "move.w $10(a2,sp.l),d0");
    }

    #[test]
    fn e12_01_size_suffix() {
        check(&[0x1001], 0, "move.b d1,d0");
        check(&[0x3001], 0, "move.w d1,d0");
        check(&[0x2001], 0, "move.l d1,d0");
        check(&[0x2041], 0, "movea.l d1,a0");
        check(&[0x203C, 0x1234, 0x5678], 0, "move.l #$12345678,d0");
    }

    #[test]
    fn e12_01_movem_list() {
        check(&[0x4CD0, 0x0F0F], 0, "movem.l (a0),d0-d3/a0-a3");
        check(&[0x4C90, 0x8001], 0, "movem.w (a0),d0/a7");
        check(&[0x4C90, 0x0005], 0, "movem.w (a0),d0/d2");
    }

    #[test]
    fn e12_01_illegal_dcw() {
        let (b, n) = dis(&[0xA000], 0);
        assert_eq!((b.s(), n), ("dc.w $a000", 1));
        let (b, n) = dis(&[0xF123, 0x1111], 0);
        assert_eq!((b.s(), n), ("dc.w $f123", 1));
    }

    #[test]
    fn e12_01_no_alloc() {
        let src = include_str!("disasm.rs");
        assert!(!src.contains(concat!("Str", "ing")));
        assert!(!src.contains(concat!("Ve", "c<")));
        assert!(!src.contains(concat!("Ve", "c::")));
        assert!(!src.contains(concat!("vec", "!")));
    }

    #[test]
    fn e12_02_move_w() {
        check(&[0x3011], 0, "move.w (a1),d0");
        check(&[0x32C0], 0, "move.w d0,(a1)+");
    }

    #[test]
    fn e12_02_moveq() {
        check(&[0x7001], 0, "moveq #1,d0");
        check(&[0x72FF], 0, "moveq #-1,d1");
    }

    #[test]
    fn e12_02_lea() {
        check(&[0x41F9, 0x0012, 0x3456], 0, "lea $123456.l,a0");
        check(&[0x4FEF, 0x0008], 0, "lea $8(sp),sp");
        check(&[0x4850], 0, "pea (a0)");
    }

    #[test]
    fn e12_02_addq() {
        check(&[0x5240], 0, "addq.w #1,d0");
        check(&[0x5148], 0, "subq.w #8,a0");
        check(&[0x0640, 0x0010], 0, "addi.w #$10,d0");
        check(&[0xB080], 0, "cmp.l d0,d0");
        check(&[0x4840], 0, "swap d0");
        check(&[0x48C0], 0, "ext.l d0");
        check(&[0x4E56, 0xFFF8], 0, "link a6,#-8");
        check(&[0x4E5E], 0, "unlk a6");
        check(&[0xC141], 0, "exg d0,d1");
        check(&[0x4240], 0, "clr.w d0");
        check(&[0x4440], 0, "neg.w d0");
        check(&[0xD0C1], 0, "adda.w d1,a0");
        check(&[0xD141], 0, "addx.w d1,d0");
        check(&[0x0188, 0x0010], 0, "movep.w d0,$10(a0)");
        check(&[0x01C8, 0x0010], 0, "movep.l d0,$10(a0)");
    }

    #[test]
    fn e12_02_movem_pre() {
        check(&[0x48E7, 0xFF00], 0, "movem.l d0-d7,-(sp)");
        check(&[0x4CDF, 0x00FF], 0, "movem.l (sp)+,d0-d7");
    }

    #[test]
    fn e12_02_mulu_divs() {
        check(&[0xC0C1], 0, "mulu.w d1,d0");
        check(&[0x81C1], 0, "divs.w d1,d0");
        check(&[0x80C1], 0, "divu.w d1,d0");
        check(&[0xC1C1], 0, "muls.w d1,d0");
    }

    #[test]
    fn e12_03_and_w() {
        check(&[0xC041], 0, "and.w d1,d0");
        check(&[0xC340], 0, "exg d1,d0");
    }

    #[test]
    fn e12_03_lsl_imm() {
        check(&[0xE548], 0, "lsl.w #2,d0");
        check(&[0xE068], 0, "lsr.w d0,d0");
        check(&[0xE108], 0, "lsl.b #8,d0");
    }

    #[test]
    fn e12_03_asr_mem() {
        check(&[0xE0D0], 0, "asr.w (a0)");
        check(&[0xE7D0], 0, "rol.w (a0)");
    }

    #[test]
    fn e12_03_btst_imm() {
        check(&[0x0800, 0x0003], 0, "btst #3,d0");
        check(&[0x0300], 0, "btst d1,d0");
        check(&[0x8001], 0, "or.b d1,d0");
        check(&[0x4600], 0, "not.b d0");
        check(&[0xB340], 0, "eor.w d1,d0");
    }

    #[test]
    fn e12_03_abcd() {
        check(&[0xC101], 0, "abcd d1,d0");
        check(&[0xC109], 0, "abcd -(a1),-(a0)");
        check(&[0x8101], 0, "sbcd d1,d0");
        check(&[0x4800], 0, "nbcd d0");
    }

    #[test]
    fn e12_04_beq() {
        check(&[0x6704], 0x1000, "beq.s $1006");
        check(&[0x66FE], 0x1000, "bne.s $1000");
    }

    #[test]
    fn e12_04_bra_w() {
        check(&[0x6000, 0x0010], 0x1000, "bra.w $1012");
        check(&[0x6100, 0xFFF0], 0x1000, "bsr.w $ff2");
    }

    #[test]
    fn e12_04_dbf() {
        check(&[0x51C8, 0xFFFC], 0x1000, "dbf d0,$ffe");
        check(&[0x56C0], 0, "sne d0");
    }

    #[test]
    fn e12_04_jsr_abs() {
        check(&[0x4EB9, 0x0000, 0x1234], 0, "jsr $1234.l");
        check(&[0x4ED0], 0, "jmp (a0)");
    }

    #[test]
    fn e12_04_rts_nop() {
        check(&[0x4E75], 0, "rts");
        check(&[0x4E71], 0, "nop");
        check(&[0x4E77], 0, "rtr");
        check(&[0x4E73], 0, "rte");
        check(&[0x4E72, 0x2700], 0, "stop #$2700");
        check(&[0x4E70], 0, "reset");
        check(&[0x4E76], 0, "trapv");
        check(&[0x4AFC], 0, "illegal");
        check(&[0x4E7A], 0, "dc.w $4e7a");
    }

    #[test]
    fn e12_04_trap() {
        check(&[0x4E4F], 0, "trap #15");
        check(&[0x4180], 0, "chk.w d0,d0");
    }

    #[test]
    fn e12_04_move_sr() {
        check(&[0x40C0], 0, "move sr,d0");
        check(&[0x46C0], 0, "move d0,sr");
        check(&[0x44C0], 0, "move d0,ccr");
        check(&[0x4E60], 0, "move a0,usp");
        check(&[0x4E68], 0, "move usp,a0");
        check(&[0x027C, 0xF8FF], 0, "andi #$f8ff,sr");
        check(&[0x003C, 0x0003], 0, "ori #$3,ccr");
        check(&[0x4AC0], 0, "tas d0");
    }
}
