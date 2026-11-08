# puce-m68k

Cycle-accurate Motorola 68000 CPU core: `no_std`, no `unsafe`, no dependency.

```rust
use puce_m68k::bus::Bus;
use puce_m68k::cpu::Cpu;

struct Ram(Vec<u8>);

impl Bus for Ram {
    fn read(&mut self, addr: u32, _fc: u8, _uds: bool, _lds: bool) -> u16 {
        let a = addr as usize & 0xFFFE;
        u16::from_be_bytes([self.0[a], self.0[a + 1]])
    }
    fn write(&mut self, addr: u32, _fc: u8, uds: bool, lds: bool, data: u16) {
        let a = addr as usize & 0xFFFE;
        if uds {
            self.0[a] = (data >> 8) as u8;
        }
        if lds {
            self.0[a + 1] = data as u8;
        }
    }
}

let mut ram = Ram(vec![0; 0x10000]);
// vectors: SSP = 0x8000, PC = 0x100; program: moveq #5,d0 ; stop #$2700
ram.0[0..8].copy_from_slice(&[0, 0, 0x80, 0, 0, 0, 0x01, 0x00]);
ram.0[0x100..0x106].copy_from_slice(&[0x70, 0x05, 0x4E, 0x72, 0x27, 0x00]);

let mut cpu = Cpu::new();
cpu.reset();
cpu.step(&mut ram);                 // reset sequence: 40 clocks
let clocks = cpu.step(&mut ram);    // moveq: 4 clocks
assert_eq!(clocks, 4);
assert_eq!(cpu.regs.d[0], 5);
```

- `Cpu::step` runs one instruction and returns its clocks (wait states included); `Cpu::tick` and
  `Cpu::step_clock` advance one clock for lock-step emulation.
- `Bus::wait_clocks`, `ipl`, `int_ack` and `on_tick` model wait states, interrupts and peripheral clocks.
- Verified against the SingleStepTests/m68000 oracle: see `docs/SUITES_REPORT.md` at the root of the repository.
- Integration guide: `docs/specs/32_REUSE_GUIDE.md`. Workspace overview: the root `README.md`.

License: MIT OR Apache-2.0.
