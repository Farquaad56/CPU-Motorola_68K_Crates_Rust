//! Micro-cycle engine: plans of tokens executed one clock at a time.

pub(crate) mod mcycle;
pub mod plan;

use crate::pins::Pins;
use crate::regs::{Regs, S};
use mcycle::{bus_pins, idle_pins, BusKind};
use plan::{AddrSrc, Plan, Tok};

#[cfg(feature = "inspect")]
use crate::probe::{BusEvent, Journal};

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

    /// Hooks (`H`) and empty internal cycles take no clock: pass over them.
    fn skip_hooks(&mut self) {
        while let Some(t) = self.plan.get(self.idx) {
            if matches!(t, Tok::H(_) | Tok::N(0)) {
                self.idx += 1;
            } else {
                break;
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
    pub(crate) fn current(&self, regs: &Regs) -> Option<Cur> {
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
        // Addresses are resolved in E05; until then every source maps to PC (+2 for the
        // second word of a long access).
        let base = match src {
            AddrSrc::Pc => regs.pc,
            AddrSrc::EaPlus2 => regs.pc.wrapping_add(2),
            _ => regs.pc,
        };
        let addr = base.wrapping_add(if self.clk >= 4 { 2 } else { 0 });
        let (uds, lds) = if byte {
            let odd = addr & 1 != 0;
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
    pub(crate) fn step(&mut self, pins: Pins, regs: &mut Regs) -> Pins {
        self.clocks += 1;
        let Some(cur) = self.current(regs) else {
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
        let out = bus_pins(kind, cur.clock, cur.addr, cur.fc, cur.uds, cur.lds, 0);
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
            if cur.clock == 3 && matches!(self.plan.get(self.idx), Some(Tok::Pf)) {
                regs.ir = regs.irc;
                regs.irc = pins.data();
                regs.pc = regs.pc.wrapping_add(2);
            }
            self.clk += 1;
            if self.clk >= cur.len {
                self.advance_token();
            }
        }
        out
    }
}
