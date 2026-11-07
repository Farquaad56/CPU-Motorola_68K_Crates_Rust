//! Speed of the core. `e15_01_bench_runs` is a smoke test; the 50 million clock measurement is
//! `#[ignore]`d: `cargo test --release -p puce-m68k-machines e15_01_ -- --ignored --nocapture`.

use puce_m68k::cpu::Cpu;
use puce_m68k::state::CpuState;
use puce_m68k_machines::flat::FlatRam;
use std::time::Instant;

/// Loads `moveq #1,d2 ; loop: move.l d0,d1 ; add.l d2,d0 ; bne.s loop` at 0x100 and runs it for
/// at least `clocks` clocks; returns (clocks, instructions, final D0).
fn bench_loop(clocks: u64) -> (u64, u64, u32) {
    let mut ram = FlatRam::new();
    for (i, w) in [0x7401u16, 0x2200, 0xD082, 0x66FA].iter().enumerate() {
        ram.poke_word(0x100 + 2 * i as u32, *w);
    }
    let mut cpu = Cpu::new();
    cpu.load_state(&CpuState {
        a: [0; 7],
        d: [0; 8],
        usp: 0,
        ssp: 0x1000,
        sr: 0x2700,
        pc: 0x104,
        prefetch: [ram.peek_word(0x100), ram.peek_word(0x102)],
    });
    let (mut c, mut n) = (0u64, 0u64);
    while c < clocks {
        let k = cpu.step(&mut ram);
        assert!(k > 0, "the loop must not stop");
        c += u64::from(k);
        n += 1;
    }
    (c, n, cpu.state().d[0])
}

#[test]
fn e15_01_bench_runs() {
    let (c, n, d0) = bench_loop(1000);
    assert!((1000..1000 + 22).contains(&c), "{c}");
    assert!(n > 100);
    // one turn = 3 instructions = 22 clocks; d0 counts the turns
    assert!(d0 > 40, "{d0}");
}

#[test]
#[ignore = "50 million clocks: run in release with --ignored"]
fn e15_01_bench_50m() {
    let t0 = Instant::now();
    let (c, n, _) = bench_loop(50_000_000);
    let s = t0.elapsed().as_secs_f64();
    let mhz = c as f64 / s / 1e6;
    println!("bench: {c} clocks, {n} instructions in {s:.3} s = {mhz:.2} MHz equivalent");
    if !cfg!(debug_assertions) {
        assert!(mhz >= 8.0, "{mhz:.2} MHz is below the 8 MHz target");
    }
}
