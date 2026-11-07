# puce-m68k-wasm

`puce-m68k` compiled to WebAssembly: a browser demo that needs no server.

The crate exposes `Machine` (flat RAM of 1 or 16 MiB + the cycle-accurate core). All the
computation stays in Rust; the page (`demo/index.html`) only displays what the module returns.

| Method | Effect |
|---|---|
| `new Machine(ram_mib)` | 1 (or less) gives 1 MiB, more gives 16 MiB |
| `load(bytes)` | binary at address 0, SSP and PC read from vectors 0 and 1; throws if it does not fit |
| `reset()` | registers reloaded from the vectors, RAM kept |
| `step()` | one instruction, returns its clocks (0 = nothing ran: illegal opcode or halted) |
| `step_clock()` | one clock: -1 nothing runs, 0 mid-instruction, 1 instruction completed |
| `regs_json()` | registers, flags, `instrs`, `clocks` |
| `disasm(addr, n)` | `n` rows `{addr, words, text, len}` |
| `mem_hex(base)` | 256 bytes as 512 hex digits |
| `buslog_json()` | last 32 bus cycles, newest first |

## Build the demo

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli        # same version as the wasm-bindgen crate (see Cargo.lock)
cargo build -p puce-m68k-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir crates/puce-m68k-wasm/demo/pkg \
    target/wasm32-unknown-unknown/release/puce_m68k_wasm.wasm
```

Then serve the `demo` folder with any static HTTP server (ES modules and WebAssembly are not
loaded from `file://`), for example:

```
python -m http.server 8000 --directory crates/puce-m68k-wasm/demo
```

and open <http://127.0.0.1:8000/>. Drop a raw `.bin` (first long = SSP, second long = PC) on the page,
then Step, Clock step or Run.

`demo/pkg/` is a build product and is not committed.

## Tests

The logic of `Machine` is plain Rust and tested natively:

```
cargo test -p puce-m68k-wasm
cargo build -p puce-m68k-wasm --target wasm32-unknown-unknown
```
