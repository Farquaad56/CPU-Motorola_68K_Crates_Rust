from _dsl import T

DONE = "FAIT par Claude lors de la creation du projet (2026-11-07): le code existe deja et passe `cargo test`. Hermes ne refait rien; l'utilisateur lance `task.py check` puis `approve` au GATE E01-07."

T("E01-01", "Squelette du crate puce-m68k", kind="code", size=40,
  context="Crate bibliotheque reutilisable `puce-m68k` (68000 cycle perfect): `no_std`, sans alloc, sans unsafe, sans dependance (D-009). " + DONE,
  embed=["01_ARCHITECTURE.md#2. Workspace Cargo", "01_ARCHITECTURE.md#3. Arborescence"],
  spec="Deja cree: `crates/puce-m68k/Cargo.toml` (features inspect, disasm, std), `src/lib.rs` (attributs, `VERSION`, modules commentes).",
  write=["crates/puce-m68k/Cargo.toml", "crates/puce-m68k/src/lib.rs", "Cargo.lock"],
  tests=[("version_const", "`VERSION` est non vide et egal a `0.1.0`"), ("no_std_attr", "`src/lib.rs` contient `#![no_std]` et `#![forbid(unsafe_code)]`")],
  accept=["cargo build -p puce-m68k", "cargo build -p puce-m68k --features inspect,disasm", "cargo test -p puce-m68k e01_01_"])

T("E01-02", "Squelette du crate puce-m68k-machines", kind="code", size=30, crate="puce-m68k-machines",
  context="Bibliotheque `std` des machines d'essai (D-006). " + DONE,
  spec="Deja cree: `Cargo.toml`, `src/lib.rs` (`pub mod flat; pub mod session; pub mod ssc;` + fichiers vides documentes, `VERSION`).",
  write=["crates/puce-m68k-machines/**", "Cargo.lock"],
  tests=[("version", "VERSION non vide")],
  accept=["cargo build -p puce-m68k-machines", "cargo test -p puce-m68k-machines e01_02_"])

T("E01-03", "Squelette du binaire puce-m68k-cli (m68k)", kind="code", size=70, crate="puce-m68k-cli",
  context="Binaire `m68k` (clap): sous-commandes `run`, `ssc`, `disasm`, `fetch` declarees, non implementees. " + DONE,
  spec="Deja cree: `src/main.rs` (Cli, Cmd), `src/commands.rs` (stubs `Err(\"not implemented\")`).",
  write=["crates/puce-m68k-cli/**", "Cargo.lock"],
  tests=[("parse_run", "`Cli::try_parse_from([\"m68k\",\"run\",\"x.bin\"])` donne Cmd::Run"), ("unknown_cmd", "une sous-commande inconnue echoue")],
  accept=["cargo build -p puce-m68k-cli", "cargo test -p puce-m68k-cli e01_03_", "cargo run -q -p puce-m68k-cli -- --help"])

T("E01-04", "Squelette du binaire puce-m68k-web (m68k-web)", kind="code", size=60, crate="puce-m68k-web",
  context="Binaire `m68k-web` (serveur du frontend): analyse des arguments seulement. " + DONE,
  embed=["01_ARCHITECTURE.md#5. Frontend"],
  spec="Deja cree: `src/args.rs` (`parse_args`: `--port`, `--roms`), `src/main.rs`.",
  write=["crates/puce-m68k-web/**", "Cargo.lock"],
  tests=[("defaults", "port 8080, roms `tests/roms`"), ("port_ok", "`--port 9000`"), ("port_bad", "`--port abc` -> Err"), ("unknown_flag", "`--foo` -> Err")],
  accept=["cargo build -p puce-m68k-web", "cargo test -p puce-m68k-web e01_04_"])

T("E01-05", "Squelette du crate puce-m68k-wasm", kind="code", size=20, crate="puce-m68k-wasm",
  context="Crate `cdylib` + `rlib` pour la demo WebAssembly (wasm-bindgen ajoute en E14). " + DONE,
  spec="Deja cree: `Cargo.toml`, `src/lib.rs` (`VERSION`).",
  write=["crates/puce-m68k-wasm/**", "Cargo.lock"],
  tests=[("version", "VERSION non vide")],
  accept=["cargo build -p puce-m68k-wasm", "cargo test -p puce-m68k-wasm e01_05_"])

T("E01-06", "Scripts de CI et d'hygiene", kind="tooling", size=60,
  context="Scripts `tools/ci.sh|ps1`, `tools/hygiene.sh|ps1`, `tools/hygiene_selftest.sh` adaptes du crate Z80 (formatage, clippy, tests, hygiene du coeur). " + DONE,
  spec="Deja cree dans `tools/`. L'hygiene verifie `crates/puce-m68k/src`: pas de `unsafe`, f32/f64, `std::`, `println!`, `unwrap()` hors tests, non-ASCII, `TODO` sans `UNKNOWN - to confirm`.",
  write=["tools/**"], tests=[],
  accept=["bash tools/hygiene.sh", "bash tools/hygiene_selftest.sh"])

T("E01-07", "GATE E01 - workspace valide", gate=True, kind="gate", size=0,
  context="Validation humaine du squelette (5 crates, CI, hygiene, docs, specs, outils Kanban).",
  user_cmds=["cargo fmt --all -- --check", "cargo clippy --workspace --all-targets -- -D warnings", "cargo test --workspace", "python docs/tools/build_tasks.py --stats"],
  accept=[])
