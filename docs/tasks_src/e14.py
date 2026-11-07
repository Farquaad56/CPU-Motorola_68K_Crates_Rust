from _dsl import T

T("E14-01", "Liaison wasm-bindgen du coeur", kind="code", size=120, crate="puce-m68k-wasm",
  context="Expose le coeur a JavaScript (demo sans serveur). Dependance autorisee: `wasm-bindgen` (cible wasm32 seulement).",
  spec="`crates/puce-m68k-wasm/src/lib.rs`: struct `Machine` (RAM plate 16 Mio ou 1 Mio configurable, `Cpu`), methodes `load(bytes)`, `step()`, `step_clock()`, `regs_json()`, `disasm(addr, n)`, `mem_hex(base)`, `buslog_json()`. Les calculs restent dans le crate; le JS ne fait que de l'affichage.",
  write=["crates/puce-m68k-wasm/src/lib.rs", "crates/puce-m68k-wasm/Cargo.toml", "Cargo.lock"],
  tests=[("native_step", "la logique `Machine` est testable en natif: step d'un NOP avance le PC"), ("regs_json", "regs_json contient pc"), ("disasm_rows", "disasm(0, 3) = 3 lignes"), ("load_bounds", "binaire trop grand -> erreur"), ("buslog", "apres un step le journal n'est pas vide")],
  accept=["cargo test -p puce-m68k-wasm e14_01_", "cargo build -p puce-m68k-wasm --target wasm32-unknown-unknown"])
T("E14-02", "Demo WebAssembly (page statique Tailwind)", kind="ui", size=120, crate="puce-m68k-wasm",
  context="Page `crates/puce-m68k-wasm/demo/index.html` reprenant les panneaux utiles de la page serveur, chargeant le module wasm.",
  spec="Registres, desassembleur, memoire, journal de bus, step/step horloge/run, chargement de binaire par glisser-deposer, FR/EN. Document le build (`wasm-bindgen --target web`) dans `crates/puce-m68k-wasm/README.md`.",
  write=["crates/puce-m68k-wasm/demo/index.html", "crates/puce-m68k-wasm/README.md", "crates/puce-m68k-wasm/tests/demo_contract.rs"],
  tests=[("title", "titre Motorola 68000 Emulator"), ("loads_wasm", "le HTML reference le module wasm"), ("tailwind", "Tailwind charge")],
  accept=["cargo test -p puce-m68k-wasm demo_contract"])
T("E14-03", "GATE E14 - demo WASM", gate=True, kind="gate", size=0,
  context="Validation humaine de la demo.",
  user_cmds=["cargo build -p puce-m68k-wasm --target wasm32-unknown-unknown --release", "ouvrir la demo dans un navigateur et executer un programme"], accept=[])
