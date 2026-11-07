//! WebAssembly bindings for puce-m68k: the core, a flat RAM and JSON views for a page.
//!
//! [`Machine`] is plain Rust and testable natively; on `wasm32` it is also exported to
//! JavaScript with `wasm-bindgen`. All the computation (registers, disassembly, bus cycles)
//! stays in the crate: the page only displays the JSON and hex strings returned here.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use puce_m68k::bus::Bus;
use puce_m68k::cpu::Cpu;
use puce_m68k::disasm::disasm as disassemble;
use puce_m68k::probe::BusEvent;
use puce_m68k::state::CpuState;
use std::fmt::Write;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bus cycles kept by [`Machine::buslog_json`].
const BUSLOG_LEN: usize = 32;

/// Flat RAM of a power-of-two size (addresses wrap).
struct Ram {
    mem: Vec<u8>,
    mask: u32,
}

impl Ram {
    fn index(&self, addr: u32) -> usize {
        (addr & self.mask & !1) as usize
    }

    fn word(&self, addr: u32) -> u16 {
        let a = self.index(addr);
        u16::from_be_bytes([self.mem[a], self.mem[a + 1]])
    }
}

impl Bus for Ram {
    fn read(&mut self, addr: u32, _fc: u8, uds: bool, lds: bool) -> u16 {
        let a = self.index(addr);
        let hi = if uds { self.mem[a] } else { 0 };
        let lo = if lds { self.mem[a + 1] } else { 0 };
        u16::from_be_bytes([hi, lo])
    }

    fn write(&mut self, addr: u32, _fc: u8, uds: bool, lds: bool, data: u16) {
        let a = self.index(addr);
        if uds {
            self.mem[a] = (data >> 8) as u8;
        }
        if lds {
            self.mem[a + 1] = data as u8;
        }
    }
}

/// A 68000 with its RAM.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Machine {
    cpu: Cpu,
    ram: Ram,
    log: Vec<BusEvent>,
    instrs: u64,
    clocks: u64,
}

fn kind_char(k: u8) -> &'static str {
    match k {
        0 => "n",
        1 => "w",
        3 => "t",
        4 => "re",
        5 => "we",
        _ => "r",
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl Machine {
    /// A machine with `ram_mib` MiB of RAM: 1 or less gives 1 MiB, more gives 16 MiB.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(ram_mib: u32) -> Machine {
        let len = if ram_mib <= 1 {
            1usize << 20
        } else {
            16usize << 20
        };
        let mut m = Machine {
            cpu: Cpu::new(),
            ram: Ram {
                mem: vec![0; len],
                mask: (len - 1) as u32,
            },
            log: Vec::new(),
            instrs: 0,
            clocks: 0,
        };
        m.reset_registers();
        m
    }

    /// RAM size in bytes.
    pub fn ram_size(&self) -> u32 {
        self.ram.mem.len() as u32
    }

    /// Crate version.
    pub fn version(&self) -> String {
        VERSION.to_string()
    }

    fn reset_registers(&mut self) {
        let long = |r: &Ram, a: u32| (u32::from(r.word(a)) << 16) | u32::from(r.word(a + 2));
        let (ssp, pc) = (long(&self.ram, 0), long(&self.ram, 4));
        self.cpu.load_state(&CpuState {
            a: [0; 7],
            d: [0; 8],
            usp: 0,
            ssp,
            sr: 0x2700,
            pc: pc.wrapping_add(4),
            prefetch: [self.ram.word(pc), self.ram.word(pc.wrapping_add(2))],
        });
        self.cpu.clear_journal();
        self.log.clear();
        self.instrs = 0;
        self.clocks = 0;
    }

    /// Loads a binary at address 0 (RAM cleared first); SSP and PC come from vectors 0 and 1.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.ram.mem.len() {
            return Err(format!(
                "binary of {} bytes does not fit in {} bytes of RAM",
                bytes.len(),
                self.ram.mem.len()
            ));
        }
        self.ram.mem.fill(0);
        self.ram.mem[..bytes.len()].copy_from_slice(bytes);
        self.reset_registers();
        Ok(())
    }

    /// Reloads the registers from the vectors (the RAM is kept).
    pub fn reset(&mut self) {
        self.reset_registers();
    }

    fn absorb(&mut self) {
        for e in self.cpu.journal() {
            self.log.insert(0, *e);
        }
        self.log.truncate(BUSLOG_LEN);
        self.cpu.clear_journal();
    }

    /// Executes one instruction; returns its clocks (0 when nothing ran).
    pub fn step(&mut self) -> u32 {
        let n = self.cpu.step(&mut self.ram);
        self.absorb();
        if n > 0 {
            self.instrs += 1;
            self.clocks += u64::from(n);
        }
        n
    }

    /// Executes one clock; returns -1 when nothing runs, 1 when an instruction completed, else 0.
    pub fn step_clock(&mut self) -> i32 {
        let r = self.cpu.step_clock(&mut self.ram);
        self.absorb();
        match r {
            None => -1,
            Some(done) => {
                self.clocks += 1;
                if done {
                    self.instrs += 1;
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Instructions executed since the last load.
    pub fn instrs(&self) -> f64 {
        self.instrs as f64
    }

    /// Clocks executed since the last load.
    pub fn clocks(&self) -> f64 {
        self.clocks as f64
    }

    /// Registers as a JSON object (`d`, `a` with `a[7]` the active stack, `usp`, `ssp`, `sr`, `pc`,
    /// `ir`, `irc`, flags, `imask`, `instrs`, `clocks`).
    pub fn regs_json(&self) -> String {
        let r = &self.cpu.regs;
        let mut a = [0u32; 8];
        a[..7].copy_from_slice(&r.a);
        a[7] = r.a7();
        let list = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
        let f = |m: u16| u8::from(r.sr & m != 0);
        format!(
            "{{\"d\":[{}],\"a\":[{}],\"usp\":{},\"ssp\":{},\"sr\":{},\"pc\":{},\"ir\":{},\"irc\":{},\
             \"x\":{},\"n\":{},\"z\":{},\"v\":{},\"c\":{},\"s\":{},\"t\":{},\"imask\":{},\
             \"instrs\":{},\"clocks\":{}}}",
            list(&r.d),
            list(&a),
            r.usp,
            r.ssp,
            r.sr,
            r.pc.wrapping_sub(4),
            r.ir,
            r.irc,
            f(0x10),
            f(0x08),
            f(0x04),
            f(0x02),
            f(0x01),
            f(0x2000),
            f(0x8000),
            (r.sr >> 8) & 7,
            self.instrs,
            self.clocks
        )
    }

    /// `n` disassembled instructions from `addr`: `[{"addr","words":[..],"text","len"}]`.
    pub fn disasm(&self, addr: u32, n: u32) -> String {
        let mut out = String::from("[");
        let mut a = addr & self.ram.mask & !1;
        for i in 0..n {
            let words: [u16; 5] =
                std::array::from_fn(|k| self.ram.word(a.wrapping_add(2 * k as u32)));
            let mut text = String::new();
            let len = disassemble(&words, a, &mut text).clamp(1, 5);
            if i > 0 {
                out.push(',');
            }
            let w: Vec<String> = words[..len].iter().map(u16::to_string).collect();
            let _ = write!(
                out,
                "{{\"addr\":{a},\"words\":[{}],\"text\":\"{}\",\"len\":{len}}}",
                w.join(","),
                text.replace('\\', "\\\\").replace('"', "\\\"")
            );
            a = a.wrapping_add(2 * len as u32) & self.ram.mask;
        }
        out.push(']');
        out
    }

    /// 256 bytes of memory from `base` as 512 hex digits.
    pub fn mem_hex(&self, base: u32) -> String {
        let mut s = String::with_capacity(512);
        for i in 0..256u32 {
            let a = ((base.wrapping_add(i)) & self.ram.mask) as usize;
            let _ = write!(s, "{:02x}", self.ram.mem[a]);
        }
        s
    }

    /// The last 32 bus cycles, newest first: `[{"kind","addr","data","fc","uds","lds","clocks"}]`.
    pub fn buslog_json(&self) -> String {
        let mut out = String::from("[");
        for (i, e) in self.log.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"kind\":\"{}\",\"addr\":{},\"data\":{},\"fc\":{},\"uds\":{},\"lds\":{},\"clocks\":{}}}",
                kind_char(e.kind),
                e.addr,
                e.data,
                e.fc,
                u8::from(e.uds),
                u8::from(e.lds),
                e.clocks
            );
        }
        out.push(']');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Image: SSP 0x1000, PC 0x100, `code` at 0x100.
    fn image(code: &[u16]) -> Vec<u8> {
        let mut v = vec![0u8; 0x100];
        v[2] = 0x10;
        v[6] = 0x01;
        for w in code {
            v.extend_from_slice(&w.to_be_bytes());
        }
        v
    }

    fn field(json: &str, key: &str) -> u64 {
        let at = json.find(key).unwrap() + key.len();
        json[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap()
    }

    #[test]
    fn e01_05_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn e14_01_native_step() {
        let mut m = Machine::new(1);
        m.load(&image(&[0x4E71, 0x4E71])).unwrap();
        assert_eq!(field(&m.regs_json(), "\"pc\":"), 0x100);
        assert_eq!(m.step(), 4);
        assert_eq!(field(&m.regs_json(), "\"pc\":"), 0x102);
        assert_eq!((m.instrs(), m.clocks()), (1.0, 4.0));
        // clock stepping: the instruction completes on the 4th clock
        let r: Vec<i32> = (0..4).map(|_| m.step_clock()).collect();
        assert_eq!(r, [0, 0, 0, 1]);
        assert_eq!(field(&m.regs_json(), "\"pc\":"), 0x104);
    }

    #[test]
    fn e14_01_regs_json() {
        let mut m = Machine::new(16);
        m.load(&image(&[0x7005])).unwrap();
        m.step();
        let j = m.regs_json();
        for k in [
            "\"d\":[5,0,0,0,0,0,0,0]",
            "\"pc\":258",
            "\"ssp\":4096",
            "\"imask\":7",
            "\"s\":1",
        ] {
            assert!(j.contains(k), "{k} in {j}");
        }
        assert_eq!(m.ram_size(), 16 << 20);
        assert_eq!(Machine::new(1).ram_size(), 1 << 20);
    }

    #[test]
    fn e14_01_disasm_rows() {
        let mut m = Machine::new(1);
        m.load(&image(&[0x7001, 0x4E71, 0x4EB9, 0x0000, 0x0200]))
            .unwrap();
        let j = m.disasm(0x100, 3);
        assert_eq!(j.matches("\"addr\":").count(), 3, "{j}");
        assert!(j.contains("\"text\":\"moveq #1,d0\""), "{j}");
        assert!(
            j.contains("\"text\":\"jsr $200.l\"") && j.contains("\"len\":3"),
            "{j}"
        );
        assert_eq!(m.disasm(0, 3).matches("\"addr\":").count(), 3);
        assert_eq!(m.disasm(0, 0), "[]");
    }

    #[test]
    fn e14_01_load_bounds() {
        let mut m = Machine::new(1);
        let e = m.load(&vec![0u8; (1 << 20) + 1]).unwrap_err();
        assert!(e.contains("does not fit"), "{e}");
        assert!(m.load(&vec![0u8; 1 << 20]).is_ok());
        assert_eq!(m.mem_hex(0).len(), 512);
        let mut m = Machine::new(1);
        m.load(&image(&[0xABCD])).unwrap();
        assert!(m.mem_hex(0x100).starts_with("abcd"));
    }

    #[test]
    fn e14_01_buslog() {
        let mut m = Machine::new(1);
        assert_eq!(m.buslog_json(), "[]");
        m.load(&image(&[0x4E71])).unwrap();
        m.step();
        let j = m.buslog_json();
        assert!(
            j.starts_with("[{\"kind\":\"r\"") && j.contains("\"clocks\":4"),
            "{j}"
        );
        assert_ne!(j, "[]");
    }
}
