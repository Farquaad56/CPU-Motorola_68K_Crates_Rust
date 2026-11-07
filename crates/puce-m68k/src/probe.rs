//! Bus journal (feature `inspect`): records the bus cycles the core executes.
//!
//! Fixed size, no allocation, no effect on the behaviour of the core.

/// Capacity of the journal.
pub const JOURNAL_LEN: usize = 64;

/// One bus event, in the shape of a SingleStepTests transaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BusEvent {
    /// 0 idle, 1 write, 2 read, 3 tas, 4 re, 5 we.
    pub kind: u8,
    /// Duration in clocks, wait states included.
    pub clocks: u32,
    /// Function code.
    pub fc: u8,
    /// Address on the bus.
    pub addr: u32,
    /// Data on the bus.
    pub data: u16,
    /// UDS strobe.
    pub uds: bool,
    /// LDS strobe.
    pub lds: bool,
}

impl BusEvent {
    const ZERO: BusEvent = BusEvent {
        kind: 0,
        clocks: 0,
        fc: 0,
        addr: 0,
        data: 0,
        uds: false,
        lds: false,
    };
}

/// Fixed-size list of events with an overflow counter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Journal {
    ev: [BusEvent; JOURNAL_LEN],
    len: usize,
    overflow: u32,
}

impl Journal {
    pub(crate) const fn new() -> Self {
        Journal {
            ev: [BusEvent::ZERO; JOURNAL_LEN],
            len: 0,
            overflow: 0,
        }
    }

    pub(crate) fn push(&mut self, e: BusEvent) {
        if self.len < JOURNAL_LEN {
            self.ev[self.len] = e;
            self.len += 1;
        } else {
            self.overflow += 1;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.len = 0;
        self.overflow = 0;
    }

    pub(crate) fn as_slice(&self) -> &[BusEvent] {
        &self.ev[..self.len]
    }

    pub(crate) fn overflow(&self) -> u32 {
        self.overflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::cpu::Cpu;
    use crate::engine::plan::{AddrSrc, Plan, Tok};

    struct B {
        waits: u8,
    }

    impl Bus for B {
        fn read(&mut self, _addr: u32, _fc: u8, _uds: bool, _lds: bool) -> u16 {
            0xCAFE
        }
        fn write(&mut self, _addr: u32, _fc: u8, _uds: bool, _lds: bool, _data: u16) {}
        fn wait_clocks(&mut self, _addr: u32, _write: bool) -> u8 {
            self.waits
        }
    }

    fn run(plan: Plan, waits: u8) -> Cpu {
        let mut cpu = Cpu::new();
        cpu.regs.pc = 0x404;
        cpu.lat.ea = 0x404;
        cpu.start_plan(plan);
        let mut bus = B { waits };
        let mut n = 0;
        while !cpu.at_instruction_boundary() {
            cpu.tick_bus(&mut bus);
            n += 1;
            assert!(n < 1000);
        }
        cpu
    }

    #[test]
    fn e03_04_read_event() {
        let cpu = run(&[Tok::Rw(AddrSrc::Ea)], 0);
        assert_eq!(
            cpu.journal(),
            &[BusEvent {
                kind: 2,
                clocks: 4,
                fc: 1,
                addr: 0x404,
                data: 0xCAFE,
                uds: true,
                lds: true
            }]
        );
    }

    #[test]
    fn e03_04_wait_included() {
        let cpu = run(&[Tok::Rw(AddrSrc::Ea)], 2);
        assert_eq!(cpu.journal().len(), 1);
        assert_eq!(cpu.journal()[0].clocks, 6);
    }

    #[test]
    fn e03_04_idle_merge() {
        let cpu = run(&[Tok::N(2), Tok::N(4)], 0);
        assert_eq!(cpu.journal().len(), 1);
        assert_eq!(cpu.journal()[0].kind, 0);
        assert_eq!(cpu.journal()[0].clocks, 6);
        let cpu = run(&[Tok::N(2), Tok::Rw(AddrSrc::Ea), Tok::N(2)], 0);
        let kinds: std::vec::Vec<u8> = cpu.journal().iter().map(|e| e.kind).collect();
        assert_eq!(kinds, std::vec![0, 2, 0]);
    }

    #[test]
    fn e03_04_overflow() {
        static P: [Tok; 70] = [Tok::Rw(AddrSrc::Ea); 70];
        let cpu = run(&P, 0);
        assert_eq!(cpu.journal().len(), JOURNAL_LEN);
        assert_eq!(cpu.journal_overflow(), 6);
    }

    #[test]
    fn e03_04_clear() {
        let mut cpu = run(&[Tok::Rw(AddrSrc::Ea)], 0);
        assert!(!cpu.journal().is_empty());
        cpu.clear_journal();
        assert!(cpu.journal().is_empty());
        assert_eq!(cpu.journal_overflow(), 0);
    }

    #[test]
    fn e03_04_no_inspect_build() {
        // The crate also builds without the feature (`cargo build -p puce-m68k`).
        assert_eq!(Cpu::new().state().sr, 0);
    }
}
