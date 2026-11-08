# Changelog

## Unreleased

- CPU core: all 68000 instructions, addressing modes and the micro-cycle engine (`Cpu::tick`, `step`,
  `step_clock`), prefetch queue, wait states, interrupts, reset, STOP.
- Exceptions: illegal instruction, line A/F, TRAP, TRAPV, CHK, privilege violation, interrupts; address
  and bus errors are deferred.
- Tools: `m68k` (`run`, `disasm`, `ssc`, `bench`, `fetch`), `m68k-web` (HTTP + WebSocket debugger), WebAssembly demo.
- Tests: SingleStepTests/m68000 runner with known exceptions, JSON baseline and regression check
  (`m68k ssc --json`, `--compare`); 260224 oracle cases pass, `TAS` is the only failing file.
- Documentation: cadrage, architecture, pins/tick spec, plan grammar, SingleStepTests format, front-end protocol,
  reuse guide, suites report.
- Workspace with five crates (`puce-m68k`, `-machines`, `-cli`, `-web`, `-wasm`), CI and hygiene scripts.
- Kanban pipeline: 148 task cards (one per instruction), strict sequential chain.
