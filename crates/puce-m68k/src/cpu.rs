//! The CPU: architectural registers plus the micro-cycle engine.

use crate::bus::Bus;
use crate::engine::plan::Plan;
use crate::engine::Engine;
use crate::pins::Pins;
#[cfg(feature = "inspect")]
use crate::probe::BusEvent;
use crate::regs::Regs;
use crate::state::CpuState;

/// A Motorola 68000 core. It owns all its state and advances one clock per [`Cpu::tick`].
#[derive(Clone, Copy, Debug)]
pub struct Cpu {
    /// Architectural registers.
    pub regs: Regs,
    engine: Engine,
    /// Wait clocks still to insert in the current bus cycle (`tick_bus` only).
    waits: u8,
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
            waits: 0,
        }
    }

    /// Loads a plan and starts executing it at the next tick.
    #[allow(dead_code)] // used by the tests, then by the decoder (E05)
    pub(crate) fn start_plan(&mut self, plan: Plan) {
        self.engine.start(plan);
    }

    /// Advances one clock. `pins` carries the inputs (DTACK, DATA), the result the outputs.
    pub fn tick(&mut self, pins: Pins) -> Pins {
        self.engine.step(pins, &mut self.regs)
    }

    /// Extension word of the current instruction: IRC, before the `Pf` that consumes it.
    pub fn ext_word(&self) -> u16 {
        self.regs.irc
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

    /// Loads a state (IR = `prefetch[0]`, IRC = `prefetch[1]`) and returns to an instruction boundary.
    pub fn load_state(&mut self, s: &CpuState) {
        self.regs = Regs {
            d: s.d,
            a: s.a,
            usp: s.usp,
            ssp: s.ssp,
            sr: s.sr,
            pc: s.pc,
            ir: s.prefetch[0],
            irc: s.prefetch[1],
        };
        self.engine.start(&[]);
        self.waits = 0;
    }

    /// Current architectural state.
    pub fn state(&self) -> CpuState {
        let r = &self.regs;
        CpuState {
            d: r.d,
            a: r.a,
            usp: r.usp,
            ssp: r.ssp,
            sr: r.sr,
            pc: r.pc,
            prefetch: [r.ir, r.irc],
        }
    }

    /// One clock connected to a [`Bus`].
    ///
    /// DTACK is high except during the wait clocks the bus asked for at the start of the cycle
    /// (`Bus::wait_clocks`). `Bus::read` is called at the clock where the CPU samples data (clock 3
    /// of the cycle), `Bus::write` at the clock where the data is valid (clock 2, once DTACK is
    /// acknowledged). `Bus::on_tick` receives the output pins of every clock.
    pub fn tick_bus<B: Bus>(&mut self, bus: &mut B) {
        let cur = self.engine.current(&self.regs).filter(|c| c.kind.is_some());
        let mut pins = Pins(0);
        let mut dtack = true;
        if let Some(c) = cur {
            if c.clock == 0 {
                self.waits = bus.wait_clocks(c.addr, c.is_write());
            }
            if c.clock == 2 && self.waits > 0 {
                self.waits -= 1;
                dtack = false;
            }
            if c.clock == 3 && !c.is_write() {
                pins = pins.with_data(bus.read(c.addr, c.fc, c.uds, c.lds));
            }
        }
        let out = self.tick(pins.with_dtack(dtack));
        if let Some(c) = cur {
            if c.is_write() && c.clock == 2 && dtack {
                bus.write(c.addr, c.fc, c.uds, c.lds, out.data());
            }
        }
        bus.on_tick(out);
    }

    /// Bus events executed since the last `clear_journal` (feature `inspect`).
    #[cfg(feature = "inspect")]
    pub fn journal(&self) -> &[BusEvent] {
        self.engine.journal.as_slice()
    }

    /// Number of events that did not fit in the journal (feature `inspect`).
    #[cfg(feature = "inspect")]
    pub fn journal_overflow(&self) -> u32 {
        self.engine.journal.overflow()
    }

    /// Empties the journal and its overflow counter (feature `inspect`).
    #[cfg(feature = "inspect")]
    pub fn clear_journal(&mut self) {
        self.engine.journal.clear();
    }

    /// `n` calls of [`Cpu::tick_bus`].
    pub fn step_clocks<B: Bus>(&mut self, bus: &mut B, n: u32) {
        for _ in 0..n {
            self.tick_bus(bus);
        }
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

    fn sample_state() -> CpuState {
        CpuState {
            d: [1, 2, 3, 4, 5, 6, 7, 8],
            a: [10, 11, 12, 13, 14, 15, 16],
            usp: 0x1000,
            ssp: 0x2000,
            sr: 0x2704,
            pc: 0x404,
            prefetch: [0x4E71, 0x1234],
        }
    }

    #[derive(Default)]
    struct Probe {
        reads: std::vec::Vec<(u32, u8, bool, bool)>,
        writes: std::vec::Vec<(u32, u8, bool, bool, u16)>,
        ticks: u32,
        waits: u8,
    }

    impl Bus for Probe {
        fn read(&mut self, addr: u32, fc: u8, uds: bool, lds: bool) -> u16 {
            self.reads.push((addr, fc, uds, lds));
            0xCAFE
        }
        fn write(&mut self, addr: u32, fc: u8, uds: bool, lds: bool, data: u16) {
            self.writes.push((addr, fc, uds, lds, data));
        }
        fn wait_clocks(&mut self, _addr: u32, _write: bool) -> u8 {
            self.waits
        }
        fn on_tick(&mut self, _pins: Pins) {
            self.ticks += 1;
        }
    }

    #[test]
    fn e03_03_load_roundtrip() {
        let mut cpu = Cpu::new();
        let s = sample_state();
        cpu.load_state(&s);
        assert_eq!(cpu.state(), s);
    }

    #[test]
    fn e03_03_ir_irc() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        assert_eq!(cpu.regs.ir, 0x4E71);
        assert_eq!(cpu.regs.irc, 0x1234);
    }

    #[test]
    fn e03_03_boundary_after_load() {
        let mut cpu = Cpu::new();
        cpu.start_plan(&[Tok::Pf]);
        assert!(!cpu.at_instruction_boundary());
        cpu.load_state(&sample_state());
        assert!(cpu.at_instruction_boundary());
    }

    #[test]
    fn e03_03_tick_bus_read() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        cpu.regs.sr = 0; // user mode: data FC = 1
        cpu.start_plan(&[Tok::Rw(AddrSrc::Ea)]);
        let mut bus = Probe::default();
        cpu.step_clocks(&mut bus, 4);
        assert_eq!(bus.reads, std::vec![(0x404, 1, true, true)]);
        assert!(bus.writes.is_empty());
        assert!(cpu.at_instruction_boundary());
        assert_eq!(bus.ticks, 4);
    }

    #[test]
    fn e03_03_tick_bus_wait() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        cpu.start_plan(&[Tok::Rw(AddrSrc::Ea)]);
        let mut bus = Probe {
            waits: 2,
            ..Probe::default()
        };
        let mut n = 0;
        while !cpu.at_instruction_boundary() {
            cpu.tick_bus(&mut bus);
            n += 1;
            assert!(n < 50);
        }
        assert_eq!(n, 6);
        assert_eq!(bus.reads.len(), 1);
    }

    #[test]
    fn e03_03_step_clocks() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        cpu.start_plan(&[Tok::N(8)]);
        let mut bus = Probe::default();
        cpu.step_clocks(&mut bus, 4);
        assert_eq!(bus.ticks, 4);
        assert_eq!(cpu.clocks(), 4);
        assert!(!cpu.at_instruction_boundary());
    }

    fn pf(cpu: &mut Cpu, waits: u8) -> (Probe, u32) {
        cpu.start_plan(&[Tok::Pf]);
        let mut bus = Probe {
            waits,
            ..Probe::default()
        };
        let mut n = 0;
        while !cpu.at_instruction_boundary() {
            cpu.tick_bus(&mut bus);
            n += 1;
            assert!(n < 50);
        }
        (bus, n)
    }

    #[test]
    fn e05_03_pf_shifts_queue() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        pf(&mut cpu, 0);
        assert_eq!(cpu.regs.ir, 0x1234);
        assert_eq!(cpu.regs.irc, 0xCAFE);
    }

    #[test]
    fn e05_03_pf_advances_pc() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        pf(&mut cpu, 0);
        assert_eq!(cpu.regs.pc, 0x406);
    }

    #[test]
    fn e05_03_pf_fc_super() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        let (bus, _) = pf(&mut cpu, 0);
        assert_eq!(bus.reads[0].1, 6);
    }

    #[test]
    fn e05_03_pf_fc_user() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        cpu.regs.sr = 0;
        let (bus, _) = pf(&mut cpu, 0);
        assert_eq!(bus.reads[0].1, 2);
    }

    #[test]
    fn e05_03_pf_address() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        let (bus, _) = pf(&mut cpu, 0);
        assert_eq!(bus.reads, std::vec![(0x404, 6, true, true)]);
    }

    #[test]
    fn e05_03_pf_4_clocks() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        assert_eq!(pf(&mut cpu, 0).1, 4);
        assert_eq!(pf(&mut cpu, 2).1, 6);
        // the queue shifted once per Pf, not once per wait clock
        assert_eq!(cpu.regs.pc, 0x408);
    }

    #[test]
    fn e05_03_ext_word() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        assert_eq!(cpu.ext_word(), 0x1234);
        pf(&mut cpu, 0);
        assert_eq!(cpu.ext_word(), 0xCAFE);
    }

    #[cfg(feature = "inspect")]
    #[test]
    fn e05_03_pf_journal() {
        let mut cpu = Cpu::new();
        cpu.load_state(&sample_state());
        pf(&mut cpu, 0);
        let j = cpu.journal();
        assert_eq!(j.len(), 1);
        assert_eq!(
            (j[0].kind, j[0].fc, j[0].addr, j[0].data),
            (2, 6, 0x404, 0xCAFE)
        );
    }
}
