//! State snapshot sent to the front end (protocol: docs/specs/31_FRONTEND_PROTOCOL.md, section 2).
//!
//! The JSON is written by hand: no dependency, strings escaped, integers only (the `ips` and `mhz`
//! fields are the only decimal numbers).

use puce_m68k::pins::Pins;
use puce_m68k::regs::Regs;
use std::fmt::Write;

/// One bus cycle of the journal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BusRow {
    /// 0 idle, 1 write, 2 read, 3 tas, 4 read with address error, 5 write with address error.
    pub kind: u8,
    /// Address on the bus.
    pub addr: u32,
    /// Data on the bus.
    pub data: u16,
    /// Function code.
    pub fc: u8,
    /// UDS strobe.
    pub uds: bool,
    /// LDS strobe.
    pub lds: bool,
    /// Duration in clocks.
    pub clocks: u32,
}

impl BusRow {
    /// Kind letter of the protocol: `n`, `r`, `w` or `t`.
    pub fn kind_char(&self) -> &'static str {
        match self.kind {
            0 => "n",
            1 | 5 => "w",
            3 => "t",
            _ => "r",
        }
    }

    /// The pins of the cycle as a `u64` (address, data, strobes, RW, FC).
    pub fn pins(&self) -> u64 {
        Pins(0)
            .with_addr(self.addr)
            .with_data(self.data)
            .with_as(self.kind != 0)
            .with_uds(self.uds)
            .with_lds(self.lds)
            .with_rw_read(!matches!(self.kind, 1 | 5))
            .with_fc(self.fc)
            .0
    }

    fn json(&self, o: &mut String, pins: bool) {
        let _ = write!(
            o,
            "{{\"kind\":\"{}\",\"addr\":{},\"data\":{},\"fc\":{},\"uds\":{},\"lds\":{},\"clocks\":{}",
            self.kind_char(),
            self.addr,
            self.data,
            self.fc,
            u8::from(self.uds),
            u8::from(self.lds),
            self.clocks
        );
        if pins {
            let _ = write!(o, ",\"pins\":\"{:#x}\"", self.pins());
        }
        o.push('}');
    }
}

/// One line of the disassembly window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisRow {
    /// Address of the instruction.
    pub addr: u32,
    /// Words of the instruction.
    pub words: Vec<u16>,
    /// Text of the instruction.
    pub text: String,
}

/// Everything the page shows, at one instant.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// `idle`, `running`, `halted`, `done` or `bp`.
    pub status: &'static str,
    /// Instructions executed since the last load.
    pub instrs: u64,
    /// Clocks elapsed since the last load.
    pub clocks: u64,
    /// Measured instructions per second.
    pub ips: f64,
    /// Measured clock rate in MHz.
    pub mhz: f64,
    /// Registers (`pc` is the prefetch address; the page shows the address of the instruction).
    pub regs: Regs,
    /// Last bus cycle.
    pub bus: Option<BusRow>,
    /// Last bus cycles, most recent first.
    pub buslog: Vec<BusRow>,
    /// Disassembly window.
    pub dis: Vec<DisRow>,
    /// Recent instructions: (address, text).
    pub trace: Vec<(u32, String)>,
    /// Stack words: (address, word).
    pub stack: Vec<(u32, u16)>,
    /// Base address of the memory window.
    pub mem_base: u32,
    /// Bytes of the memory window.
    pub mem: Vec<u8>,
    /// Addresses written recently.
    pub wlog: Vec<u32>,
    /// Console text added since the last state.
    pub console_delta: String,
    /// Breakpoints.
    pub bps: Vec<u32>,
    /// Loaded binary: (name, size).
    pub rom: Option<(String, usize)>,
}

/// Escapes a string for JSON (quotes, backslash, control characters).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o
}

fn list<T>(o: &mut String, items: &[T], mut f: impl FnMut(&mut String, &T)) {
    o.push('[');
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        f(o, it);
    }
    o.push(']');
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.2}")
    } else {
        "0".into()
    }
}

impl Snapshot {
    /// The `{"type":"state", ...}` message.
    pub fn to_json(&self) -> String {
        let r = &self.regs;
        let mut o = String::with_capacity(4096);
        let _ = write!(
            o,
            "{{\"type\":\"state\",\"status\":\"{}\",\"instrs\":{},\"clocks\":{},\"ips\":{},\"mhz\":{}",
            self.status,
            self.instrs,
            self.clocks,
            num(self.ips),
            num(self.mhz)
        );
        o.push_str(",\"regs\":{\"d\":");
        list(&mut o, &r.d, |o, v| {
            let _ = write!(o, "{v}");
        });
        o.push_str(",\"a\":");
        let mut a = [0u32; 8];
        a[..7].copy_from_slice(&r.a);
        a[7] = r.a7();
        list(&mut o, &a, |o, v| {
            let _ = write!(o, "{v}");
        });
        let b = |m: u16| u8::from(r.sr & m != 0);
        let _ = write!(
            o,
            ",\"usp\":{},\"ssp\":{},\"sr\":{},\"pc\":{},\"ir\":{},\"irc\":{},\"x\":{},\"n\":{},\"z\":{},\"v\":{},\"c\":{},\"s\":{},\"t\":{},\"imask\":{}}}",
            r.usp,
            r.ssp,
            r.sr,
            r.pc.wrapping_sub(4),
            r.ir,
            r.irc,
            b(0x10),
            b(0x08),
            b(0x04),
            b(0x02),
            b(0x01),
            b(0x2000),
            b(0x8000),
            (r.sr >> 8) & 7
        );
        o.push_str(",\"bus\":");
        match &self.bus {
            Some(row) => row.json(&mut o, true),
            None => o.push_str("null"),
        }
        o.push_str(",\"buslog\":");
        list(&mut o, &self.buslog, |o, row| row.json(o, false));
        o.push_str(",\"dis\":");
        list(&mut o, &self.dis, |o, d| {
            let _ = write!(o, "{{\"addr\":{},\"words\":", d.addr);
            list(o, &d.words, |o, w| {
                let _ = write!(o, "{w}");
            });
            let _ = write!(
                o,
                ",\"text\":\"{}\",\"len\":{}}}",
                esc(&d.text),
                d.words.len()
            );
        });
        o.push_str(",\"trace\":");
        list(&mut o, &self.trace, |o, (pc, t)| {
            let _ = write!(o, "{{\"pc\":{pc},\"text\":\"{}\"}}", esc(t));
        });
        o.push_str(",\"stack\":");
        list(&mut o, &self.stack, |o, (a, w)| {
            let _ = write!(o, "{{\"addr\":{a},\"word\":{w}}}");
        });
        let _ = write!(o, ",\"mem\":{{\"base\":{},\"hex\":\"", self.mem_base);
        for byte in &self.mem {
            let _ = write!(o, "{byte:02x}");
        }
        o.push_str("\"},\"wlog\":");
        list(&mut o, &self.wlog, |o, a| {
            let _ = write!(o, "{a}");
        });
        let _ = write!(o, ",\"console_delta\":\"{}\"", esc(&self.console_delta));
        o.push_str(",\"bps\":");
        list(&mut o, &self.bps, |o, a| {
            let _ = write!(o, "{a}");
        });
        o.push_str(",\"rom\":");
        match &self.rom {
            Some((name, size)) => {
                let _ = write!(o, "{{\"name\":\"{}\",\"size\":{size}}}", esc(name));
            }
            None => o.push_str("null"),
        }
        o.push('}');
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            status: "idle",
            mem: vec![0xAB; 256],
            ..Snapshot::default()
        }
    }

    #[test]
    fn e13_02_json_valid_regs() {
        let j = snap().to_json();
        for k in [
            "\"d\":[",
            "\"a\":[",
            "\"sr\":",
            "\"pc\":",
            "\"imask\":",
            "\"type\":\"state\"",
        ] {
            assert!(j.contains(k), "{k} in {j}");
        }
        assert!(j.starts_with('{') && j.ends_with('}'));
    }

    #[test]
    fn e13_02_escape() {
        assert_eq!(esc("a\"b\\c\nd\u{1}"), "a\\\"b\\\\c\\nd\\u0001");
        let mut s = snap();
        s.console_delta = "say \"hi\"".into();
        assert!(s.to_json().contains("\"console_delta\":\"say \\\"hi\\\"\""));
    }

    #[test]
    fn e13_02_mem_hex() {
        let j = snap().to_json();
        let at = j.find("\"hex\":\"").unwrap() + 7;
        let end = j[at..].find('"').unwrap();
        assert_eq!(end, 512);
    }

    #[test]
    fn e13_02_buslog_order() {
        let mut s = snap();
        s.buslog = vec![
            BusRow {
                kind: 2,
                addr: 2,
                clocks: 4,
                ..BusRow::default()
            },
            BusRow {
                kind: 1,
                addr: 1,
                clocks: 4,
                ..BusRow::default()
            },
        ];
        let j = s.to_json();
        let first = j.find("\"addr\":2").unwrap();
        let second = j.find("\"addr\":1").unwrap();
        assert!(first < second);
        assert!(j.contains("\"kind\":\"w\""));
    }

    #[test]
    fn e13_02_missing_rom() {
        let j = snap().to_json();
        assert!(j.contains("\"rom\":null") && j.contains("\"bus\":null"));
    }

    #[test]
    fn e13_02_pins_field() {
        let mut s = snap();
        s.bus = Some(BusRow {
            kind: 2,
            addr: 0x1234,
            data: 0xBEEF,
            fc: 5,
            uds: true,
            lds: true,
            clocks: 4,
        });
        let j = s.to_json();
        assert!(j.contains("\"pins\":\"0x"), "{j}");
    }
}
