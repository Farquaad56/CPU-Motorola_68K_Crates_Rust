//! Output pin profile of a basic 4-clock bus cycle (spec 11, section 3).
//!
//! `clock` is the 0-based clock inside the cycle (0..=3 = S0..S3 of the spec text).
//! The CPU never drives DATA during a read and never drives DTACK.

use crate::pins::Pins;

/// Direction of a basic bus cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // used by the engine (E02-06 and later)
pub(crate) enum BusKind {
    /// Read cycle.
    Read,
    /// Write cycle.
    Write,
}

/// Pins driven by the CPU at `clock` (0..=3) of a bus cycle without wait states.
#[allow(dead_code)] // used by the engine (E02-06 and later)
pub(crate) fn bus_pins(
    kind: BusKind,
    clock: u8,
    addr: u32,
    fc: u8,
    uds: bool,
    lds: bool,
    data: u16,
) -> Pins {
    let clock = clock.min(3);
    // Clock 0: address and function code, RW still high, strobes inactive.
    let mut p = Pins(0).with_addr(addr).with_fc(fc).with_rw_read(true);
    match kind {
        BusKind::Read => {
            // Clock 1: AS and data strobes active. Clocks 2 and 3: held until the cycle ends
            // (UNKNOWN - to confirm: exact half-clock release, manual chapter 5).
            if clock >= 1 {
                p = p.with_as(true).with_uds(uds).with_lds(lds);
            }
        }
        BusKind::Write => {
            // Clock 1: AS active and RW low.
            if clock >= 1 {
                p = p.with_as(true).with_rw_read(false);
            }
            // Clock 2: data on the bus.
            if clock >= 2 {
                p = p.with_data(data);
            }
            // Clock 3: data strobes active, released at the end of the cycle
            // (UNKNOWN - to confirm: manual diagrams).
            if clock >= 3 {
                p = p.with_uds(uds).with_lds(lds);
            }
        }
    }
    p
}

/// Pins between bus cycles: strobes inactive, address and FC held.
#[allow(dead_code)] // used by the engine (E02-06 and later)
pub(crate) fn idle_pins(fc: u8, prev_addr: u32) -> Pins {
    // UNKNOWN - to confirm: address/RW state during internal cycles.
    Pins(0).with_addr(prev_addr).with_fc(fc).with_rw_read(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e02_05_read_clock0() {
        let p = bus_pins(BusKind::Read, 0, 0x1234, 6, true, true, 0);
        assert_eq!(p.addr(), 0x1234);
        assert_eq!(p.fc(), 6);
        assert!(p.rw_read());
        assert!(!p.as_());
    }

    #[test]
    fn e02_05_read_clock1() {
        let p = bus_pins(BusKind::Read, 1, 0x1234, 6, true, false, 0);
        assert!(p.as_() && p.uds() && !p.lds());
        let q = bus_pins(BusKind::Read, 1, 0x1234, 6, false, true, 0);
        assert!(q.as_() && !q.uds() && q.lds());
    }

    #[test]
    fn e02_05_read_clock3_release() {
        let p = bus_pins(BusKind::Read, 3, 0x1234, 6, true, true, 0);
        assert!(p.as_() && p.uds() && p.lds());
    }

    #[test]
    fn e02_05_write_rw_low() {
        assert!(bus_pins(BusKind::Write, 0, 0x10, 5, true, true, 0xAAAA).rw_read());
        for c in 1..=3 {
            assert!(!bus_pins(BusKind::Write, c, 0x10, 5, true, true, 0xAAAA).rw_read());
        }
    }

    #[test]
    fn e02_05_write_data_clock2() {
        assert_eq!(
            bus_pins(BusKind::Write, 1, 0x10, 5, true, true, 0xBEEF).data(),
            0
        );
        assert_eq!(
            bus_pins(BusKind::Write, 2, 0x10, 5, true, true, 0xBEEF).data(),
            0xBEEF
        );
        let p = bus_pins(BusKind::Write, 3, 0x10, 5, true, false, 0xBEEF);
        assert!(p.uds() && !p.lds());
    }

    #[test]
    fn e02_05_idle() {
        let p = idle_pins(6, 0x2000);
        assert!(!p.as_() && !p.uds() && !p.lds());
        assert_eq!((p.addr(), p.fc()), (0x2000, 6));
    }
}
