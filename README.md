# CPU-Motorola_68K_Crates_Rust

A reusable, **cycle-accurate Motorola 68000 emulator core** written in Rust, with the tools used to prove it correct.
Work in progress: the workspace, test runner plan and the Hermes Kanban cards (148 tasks) are in place; the CPU itself is built card by card.

## Workspace

| Crate | Role | Binary |
|---|---|---|
| [`puce-m68k`](crates/puce-m68k) | The CPU core (`no_std`, no `unsafe`, no dependency) | - |
| [`puce-m68k-machines`](crates/puce-m68k-machines) | Flat RAM machine, SingleStepTests runner, simulation session | - |
| [`puce-m68k-cli`](crates/puce-m68k-cli) | Command-line tool | `m68k` |
| [`puce-m68k-web`](crates/puce-m68k-web) | Local web server and Tailwind browser front end (HTTP + WebSocket) | `m68k-web` |
| [`puce-m68k-wasm`](crates/puce-m68k-wasm) | The core compiled to WebAssembly (serverless demo) | - |

## Targets

- 68000 only; accuracy at the micro-cycle level (`Cpu::tick` = one clock, bus cycle = 4 clocks + DTACK wait states).
- Pins: DTACK, UDS/LDS, FC0-2, AS, R/W. Exceptions and interrupts are a deferred epic (E11).
- Oracle: [SingleStepTests/m68000](https://github.com/SingleStepTests/m68000) (317500 tests, generated from MAME's microcoded core) and `Yacht.txt` (microcode-level bus timing).

## Getting started

```
python docs/tools/fetch_refs.py            # Yacht.txt, manuals, SingleStepTests (network needed once)
python docs/tools/task.py status           # progress per epic
python docs/tools/task.py next             # next card
python docs/tools/build_tasks.py           # regenerate cards from docs/tasks_src/
cargo test --workspace
```

Method and decisions: `docs/00_CADRAGE.md`, `docs/00_METHODE.md`, `docs/01_ARCHITECTURE.md`, `docs/qa/DECISIONS.md`.
Kanban (Nous Hermes): `docs/03_KANBAN_HERMES.md`.

## License

MIT OR Apache-2.0.
