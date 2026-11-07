//! Micro-cycle engine: plans of tokens executed one clock at a time.

pub(crate) mod mcycle;
pub mod plan;

use crate::pins::Pins;
use crate::regs::{Regs, S};
use mcycle::{bus_pins, idle_pins, BusKind};
use plan::{AddrSrc, Plan, Tok};

#[cfg(feature = "inspect")]
use crate::probe::{BusEvent, Journal};

/// Internal latches shared by the engine and the instruction hooks (not architectural state).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Latch {
    /// Effective address resolved by the hooks (`AddrSrc::Ea`, `EaPlus2` and the hook-managed sources).
    pub(crate) ea: u32,
    /// Operand latch: loaded by the read tokens, stored by the write tokens.
    pub(crate) val: u32,
    /// Opcode of the instruction being executed (IR moves on with every prefetch).
    pub(crate) ird: u16,
    /// Scratch word for multi-step instructions (MOVEM: remaining mask and current register).
    pub(crate) aux: u32,
    /// Second scratch word (ADDX/SUBX: low half of a long read low-word first).
    pub(crate) aux2: u32,
}

/// What the engine does at its next clock.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cur {
    /// `None` for an internal cycle.
    pub(crate) kind: Option<BusKind>,
    /// Length of the whole token in clocks.
    pub(crate) len: u8,
    pub(crate) addr: u32,
    pub(crate) fc: u8,
    pub(crate) uds: bool,
    pub(crate) lds: bool,
    /// True for the TAS read-modify-write token.
    #[cfg_attr(not(feature = "inspect"), allow(dead_code))]
    pub(crate) tas: bool,
    /// Clock inside the current bus cycle (0..=3) for a bus token.
    pub(crate) clock: u8,
}

impl Cur {
    /// True for a write cycle.
    pub(crate) fn is_write(&self) -> bool {
        self.kind == Some(BusKind::Write)
    }
}

/// Private state of the plan executor.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Engine {
    plan: Plan,
    /// Index of the current token.
    idx: usize,
    /// Clock inside the current token (bus tokens: 0..4 or 0..8, N: 0..n).
    clk: u8,
    /// Total clocks elapsed, waits included.
    clocks: u64,
    /// Last address driven on the bus.
    last_addr: u32,
    /// Hooks passed over and not yet run by the CPU (FIFO: hp is the next, hn the end).
    hooks: [u8; 8],
    hn: u8,
    hp: u8,
    /// Bus journal (feature inspect).
    #[cfg(feature = "inspect")]
    pub(crate) journal: Journal,
    /// Clocks of the bus cycle in progress, waits included.
    #[cfg(feature = "inspect")]
    cyc_clocks: u32,
    /// Internal clocks not yet written to the journal.
    #[cfg(feature = "inspect")]
    idle_acc: u32,
}

impl Engine {
    pub(crate) const fn new() -> Self {
        Engine {
            plan: &[],
            idx: 0,
            clk: 0,
            clocks: 0,
            last_addr: 0,
            hooks: [0; 8],
            hn: 0,
            hp: 0,
            #[cfg(feature = "inspect")]
            journal: Journal::new(),
            #[cfg(feature = "inspect")]
            cyc_clocks: 0,
            #[cfg(feature = "inspect")]
            idle_acc: 0,
        }
    }

    pub(crate) fn start(&mut self, plan: Plan) {
        self.plan = plan;
        self.idx = 0;
        self.clk = 0;
        self.hn = 0;
        self.hp = 0;
        #[cfg(feature = "inspect")]
        {
            self.cyc_clocks = 0;
            self.idle_acc = 0;
        }
        self.skip_hooks();
    }

    pub(crate) fn boundary(&self) -> bool {
        self.idx >= self.plan.len()
    }

    pub(crate) fn clocks(&self) -> u64 {
        self.clocks
    }

    /// Next hook passed over and not yet run (FIFO), if any.
    pub(crate) fn pop_hook(&mut self) -> Option<u8> {
        if self.hp < self.hn {
            let k = self.hooks[self.hp as usize];
            self.hp += 1;
            if self.hp == self.hn {
                self.hp = 0;
                self.hn = 0;
            }
            Some(k)
        } else {
            None
        }
    }

    /// Hooks (`H`) are queued for the CPU, empty internal cycles dropped: neither takes a clock.
    fn skip_hooks(&mut self) {
        while let Some(t) = self.plan.get(self.idx) {
            match *t {
                Tok::H(k) => {
                    debug_assert!((self.hn as usize) < self.hooks.len(), "too many hooks");
                    if (self.hn as usize) < self.hooks.len() {
                        self.hooks[self.hn as usize] = k;
                        self.hn += 1;
                    }
                    self.idx += 1;
                }
                Tok::N(0) => self.idx += 1,
                _ => break,
            }
        }
    }

    fn advance_token(&mut self) {
        self.idx += 1;
        self.clk = 0;
        self.skip_hooks();
        // Consecutive internal cycles merge into one idle event.
        #[cfg(feature = "inspect")]
        if self.idle_acc > 0 && !matches!(self.plan.get(self.idx), Some(Tok::N(_))) {
            self.journal.push(BusEvent {
                clocks: self.idle_acc,
                ..BusEvent::default()
            });
            self.idle_acc = 0;
        }
    }

    /// Describes the next clock, or `None` when the plan is finished.
    pub(crate) fn current(&self, regs: &Regs, lat: &Latch) -> Option<Cur> {
        let supervisor = regs.sr & S != 0;
        let fc_data = if supervisor { 5 } else { 1 };
        let fc_prog = if supervisor { 6 } else { 2 };
        let tok = *self.plan.get(self.idx)?;
        let (kind, len, src, fc, byte) = match tok {
            Tok::N(n) => {
                return Some(Cur {
                    kind: None,
                    len: n,
                    addr: self.last_addr,
                    fc: fc_data,
                    uds: false,
                    lds: false,
                    tas: false,
                    clock: self.clk,
                });
            }
            Tok::H(_) => unreachable!("hooks are skipped"),
            Tok::Pf => (BusKind::Read, 4, AddrSrc::Pc, fc_prog, false),
            Tok::Rb(a) => (BusKind::Read, 4, a, fc_data, true),
            Tok::Rw(a) => (BusKind::Read, 4, a, fc_data, false),
            Tok::Rl(a) => (BusKind::Read, 8, a, fc_data, false),
            Tok::Wb(a) => (BusKind::Write, 4, a, fc_data, true),
            Tok::Ww(a) => (BusKind::Write, 4, a, fc_data, false),
            Tok::Wl(a) => (BusKind::Write, 8, a, fc_data, false),
            Tok::Tas(a) => (BusKind::Read, 4, a, fc_data, true),
        };
        // PC-relative operands are read in program space.
        let fc = if src == AddrSrc::EaProg { fc_prog } else { fc };
        // `Ea`, `EaPlus2` and the sources managed by hooks (pre/post-increment, vectors) use
        // the effective address latch. The second word of a long access is 2 bytes further.
        let base = match src {
            AddrSrc::Pc => regs.pc,
            AddrSrc::Ea | AddrSrc::EaProg => lat.ea,
            AddrSrc::EaPlus2 => lat.ea.wrapping_add(2),
            AddrSrc::An(n) => regs.an(n),
            AddrSrc::Sp => regs.a7(),
            _ => lat.ea,
        };
        // The bus carries a 24-bit even address; a byte access selects the lane with UDS/LDS.
        let raw = base.wrapping_add(if self.clk >= 4 { 2 } else { 0 });
        let addr = raw & crate::pins::ADDR_EVEN_MASK;
        let (uds, lds) = if byte {
            let odd = raw & 1 != 0;
            (!odd, odd)
        } else {
            (true, true)
        };
        Some(Cur {
            kind: Some(kind),
            len,
            addr,
            fc,
            uds,
            lds,
            tas: matches!(tok, Tok::Tas(_)),
            clock: self.clk % 4,
        })
    }

    /// Runs one clock and returns the pins driven by the CPU.
    ///
    /// A `Pf` token shifts the prefetch queue at clock 3 of its bus cycle, once DTACK is
    /// acknowledged: `IR = IRC`, `IRC = word read`, `PC += 2`.
    pub(crate) fn step(&mut self, pins: Pins, regs: &mut Regs, lat: &mut Latch) -> Pins {
        self.clocks += 1;
        let Some(cur) = self.current(regs, lat) else {
            let fc = if regs.sr & S != 0 { 5 } else { 1 };
            return idle_pins(fc, self.last_addr);
        };
        let Some(kind) = cur.kind else {
            let out = idle_pins(cur.fc, self.last_addr);
            #[cfg(feature = "inspect")]
            {
                self.idle_acc += 1;
            }
            self.clk += 1;
            if self.clk >= cur.len {
                self.advance_token();
            }
            return out;
        };
        // DTACK is sampled at clock 2 of every bus cycle: repeat the clock while it is low.
        let wait = cur.clock == 2 && !pins.dtack();
        let tok = self.plan.get(self.idx).copied();
        let wdata = match tok {
            Some(Tok::Wb(_)) => {
                let b = lat.val as u8 as u16;
                if cur.lds {
                    b
                } else {
                    b << 8
                }
            }
            Some(Tok::Ww(_)) => lat.val as u16,
            Some(Tok::Wl(_)) => {
                if self.clk >= 4 {
                    lat.val as u16
                } else {
                    (lat.val >> 16) as u16
                }
            }
            _ => 0,
        };
        let out = bus_pins(kind, cur.clock, cur.addr, cur.fc, cur.uds, cur.lds, wdata);
        self.last_addr = cur.addr;
        #[cfg(feature = "inspect")]
        {
            self.cyc_clocks += 1;
            if cur.clock == 3 && !wait {
                let write = cur.is_write();
                self.journal.push(BusEvent {
                    kind: if cur.tas {
                        3
                    } else if write {
                        1
                    } else {
                        2
                    },
                    clocks: self.cyc_clocks,
                    fc: cur.fc,
                    addr: cur.addr,
                    data: if write { out.data() } else { pins.data() },
                    uds: cur.uds,
                    lds: cur.lds,
                });
                self.cyc_clocks = 0;
            }
        }
        if !wait {
            if cur.clock == 3 {
                let d = pins.data();
                match tok {
                    Some(Tok::Pf) => {
                        regs.ir = regs.irc;
                        regs.irc = d;
                        regs.pc = regs.pc.wrapping_add(2);
                    }
                    Some(Tok::Rb(_)) | Some(Tok::Tas(_)) => {
                        lat.val = if cur.lds { d & 0xFF } else { d >> 8 } as u32;
                    }
                    Some(Tok::Rw(_)) => lat.val = d as u32,
                    Some(Tok::Rl(_)) => {
                        lat.val = if self.clk >= 4 {
                            (lat.val & 0xFFFF_0000) | d as u32
                        } else {
                            (d as u32) << 16
                        };
                    }
                    _ => {}
                }
            }
            self.clk += 1;
            if self.clk >= cur.len {
                self.advance_token();
            }
        }
        out
    }
}
