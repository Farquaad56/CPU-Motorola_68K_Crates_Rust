//! The CPU: architectural registers plus the micro-cycle engine.

use crate::engine::plan::Plan;
use crate::engine::Engine;
use crate::pins::Pins;
use crate::regs::Regs;

/// A Motorola 68000 core. It owns all its state and advances one clock per [`Cpu::tick`].
#[derive(Clone, Copy, Debug)]
pub struct Cpu {
    /// Architectural registers.
    pub regs: Regs,
    engine: Engine,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    /// Creates a CPU with zeroed registers and no plan to run.
    pub const fn new() -> Self {
        Cpu {
            regs: Regs {
                d: [0; 8],
                a: [0; 7],
                usp: 0,
                ssp: 0,
                sr: 0,
                pc: 0,
                ir: 0,
                irc: 0,
            },
            engine: Engine::new(),
        }
    }

    /// Loads a plan and starts executing it at the next tick.
    #[allow(dead_code)] // used by the tests, then by the decoder (E05)
    pub(crate) fn start_plan(&mut self, plan: Plan) {
        self.engine.start(plan);
    }

    /// Advances one clock. `pins` carries the inputs (DTACK, DATA), the result the outputs.
    pub fn tick(&mut self, pins: Pins) -> Pins {
        self.engine.step(pins, &self.regs)
    }

    /// True when no plan is running (the next instruction can start).
    pub fn at_instruction_boundary(&self) -> bool {
        self.engine.boundary()
    }

    /// Total clocks elapsed, wait states included.
    #[allow(dead_code)] // used by the tests, then by the runners
    pub(crate) fn clocks(&self) -> u64 {
        self.engine.clocks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::plan::{AddrSrc, Tok};

    const ACK: Pins = Pins(1 << 44);

    /// Runs the plan with DTACK high except on the 1-based ticks in `low`; returns the tick count.
    fn run(cpu: &mut Cpu, plan: Plan, low: &[u32]) -> u32 {
        cpu.start_plan(plan);
        let mut n = 0;
        while !cpu.at_instruction_boundary() {
            n += 1;
            let pins = if low.contains(&n) { Pins(0) } else { ACK };
            cpu.tick(pins);
            assert!(n < 100, "plan does not terminate");
        }
        n
    }

    #[test]
    fn e02_06_read_4_clocks() {
        let mut cpu = Cpu::new();
        assert_eq!(run(&mut cpu, &[Tok::Rw(AddrSrc::Ea)], &[]), 4);
    }

    #[test]
    fn e02_06_read_wait() {
        let mut cpu = Cpu::new();
        assert_eq!(run(&mut cpu, &[Tok::Rw(AddrSrc::Ea)], &[3, 4, 5]), 7);
    }

    #[test]
    fn e02_06_long_8_clocks() {
        let mut cpu = Cpu::new();
        assert_eq!(run(&mut cpu, &[Tok::Rl(AddrSrc::Ea)], &[]), 8);
    }

    #[test]
    fn e02_06_internal_n4() {
        let mut cpu = Cpu::new();
        cpu.start_plan(&[Tok::N(4)]);
        for _ in 0..4 {
            let out = cpu.tick(Pins(0));
            assert!(!out.as_() && !out.uds() && !out.lds());
        }
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e02_06_two_tokens() {
        let mut cpu = Cpu::new();
        assert_eq!(run(&mut cpu, &[Tok::N(2), Tok::Rw(AddrSrc::Ea)], &[]), 6);
    }

    #[test]
    fn e02_06_clock_counter() {
        let mut cpu = Cpu::new();
        let n = run(&mut cpu, &[Tok::N(2), Tok::Rw(AddrSrc::Ea)], &[5, 6]);
        assert_eq!(n, 8);
        assert_eq!(cpu.clocks(), 8);
    }
}
