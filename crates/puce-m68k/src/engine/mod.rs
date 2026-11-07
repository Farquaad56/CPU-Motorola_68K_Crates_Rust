//! Micro-cycle engine: plans of tokens executed one clock at a time.

pub(crate) mod mcycle;
pub mod plan;

use crate::pins::Pins;
use crate::regs::{Regs, S};
use mcycle::{bus_pins, idle_pins, BusKind};
use plan::{AddrSrc, Plan, Tok};

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
}

impl Engine {
    pub(crate) const fn new() -> Self {
        Engine {
            plan: &[],
            idx: 0,
            clk: 0,
            clocks: 0,
            last_addr: 0,
        }
    }

    pub(crate) fn start(&mut self, plan: Plan) {
        self.plan = plan;
        self.idx = 0;
        self.clk = 0;
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
    }

    /// Runs one clock and returns the pins driven by the CPU.
    pub(crate) fn step(&mut self, pins: Pins, regs: &Regs) -> Pins {
        self.clocks += 1;
        let supervisor = regs.sr & S != 0;
        let fc_data = if supervisor { 5 } else { 1 };
        let fc_prog = if supervisor { 6 } else { 2 };
        let Some(&tok) = self.plan.get(self.idx) else {
            return idle_pins(fc_data, self.last_addr);
        };
        // (kind, token length in clocks, base address, function code, byte access)
        let (kind, len, src, fc, byte) = match tok {
            Tok::N(n) => {
                let out = idle_pins(fc_data, self.last_addr);
                self.clk += 1;
                if self.clk >= n {
                    self.advance_token();
                }
                return out;
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
            AddrSrc::EaPlus2 => regs.pc.wrapping_add(2),
            _ => regs.pc,
        };
        let second_word = self.clk >= 4;
        let addr = base.wrapping_add(if second_word { 2 } else { 0 });
        let clock = self.clk % 4;
        let (uds, lds) = if byte {
            let odd = addr & 1 != 0;
            (!odd, odd)
        } else {
            (true, true)
        };
        // DTACK is sampled at clock 2 of every bus cycle: repeat the clock while it is low.
        let wait = clock == 2 && !pins.dtack();
        let out = bus_pins(kind, clock, addr, fc, uds, lds, 0);
        self.last_addr = addr;
        if !wait {
            self.clk += 1;
            if self.clk >= len {
                self.advance_token();
            }
        }
        out
    }
}
