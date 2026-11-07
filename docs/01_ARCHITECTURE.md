# 01 - ARCHITECTURE (crate 68000 cycle perfect)

## 1. Decisions de cadrage (detail dans docs/qa/DECISIONS.md et docs/00_CADRAGE.md)

| ID | Decision |
|---|---|
| D-001 | Perimetre: 68000 seul. |
| D-002 | Precision micro-cycle: `Cpu::tick(pins) -> pins` = 1 horloge, le moteur ordonnance par micro-cycles de 2 horloges (voir specs/11_PINS_AND_TICK.md). |
| D-003 | Broches: DTACK, UDS/LDS, FC0-2, AS, RW. Reste (exceptions, interruptions, arbitrage) = E11 differee. |
| D-005 | Frontend = serveur local Rust (HTTP + WebSocket 127.0.0.1) + page Tailwind FR/EN; aucun calcul CPU en JavaScript; demo WASM sans serveur. |
| D-006 | Machine d'essai: RAM plate + runner SingleStepTests. |
| D-007 | Oracle: SingleStepTests/m68000 (MAME) + Yacht.txt. |
| D-009 | Coeur `#![no_std]`, `#![forbid(unsafe_code)]`, sans alloc, zero dependance, entiers uniquement. |
| D-032 | Contraintes NeoGeo (specs/40_NEOGEO_CONSTRAINTS.md): E11-02, E11-05, E11-06 obligatoires; `tick(pins)` conserve; pas d'horloge dans le coeur. |
| D-010 | Une carte = une instruction. Chaine strictement sequentielle (D-020). |

## 2. Workspace Cargo

```
Cargo.toml                       [workspace] members = crates/*
crates/puce-m68k/                lib no_std (le crate reutilisable); features: inspect, disasm, std
crates/puce-m68k-machines/       lib (std): flat (RAM plate 16 Mio), ssc (runner SingleStepTests), session
crates/puce-m68k-cli/            binaire `m68k`: run | ssc | disasm | fetch (clap)
crates/puce-m68k-web/            binaire `m68k-web`: serveur HTTP+WebSocket + page embarquee (include_str!)
crates/puce-m68k-wasm/           coeur compile en WebAssembly (wasm-bindgen, ajoute en E14)
tests/singlestep/                127 fichiers .json.bin (telecharges, jamais commites), FILES.txt
docs/                            toute la documentation; docs/ref/ = references telechargees
```

## 3. Arborescence de `puce-m68k/src/`

```
lib.rs          #![no_std] #![forbid(unsafe_code)], re-exports, VERSION
pins.rs         struct Pins(u64) + accesseurs
regs.rs         struct Regs (D0-D7, A0-A6, USP, SSP, SR, PC, IR, IRC), constantes de drapeaux
bus.rs          trait Bus (confort)
cpu.rs          struct Cpu, new/tick/step/at_instruction_boundary, load_state/state (tests)
engine/         plan.rs (jetons), mcycle.rs (broches par horloge), mod.rs (etat du moteur)
decode.rs       table 65536 entrees (const fn) opcode -> instruction
ea.rs           decodage des modes d'adressage et plans d'adresse effective
insn/mod.rs     registre des instructions (RULES: masque/valeur) ; insn/<mnemonique>.rs : regles + plans + semantique d'UNE instruction
alu.rs          fonctions pures de drapeaux
exception.rs    (E11) frames d'exception, address error, interruptions
probe.rs        (feature "inspect") journal de bus
disasm.rs       (feature "disasm") desassembleur
```

## 4. Principe du moteur

1. L'instruction courante est dans IR (avec IRC = mot suivant): `decode` renvoie l'instruction et sa variante.
2. Chaque instruction = un plan de jetons (specs/12_PLAN_GRAMMAR.md); `tick` avance d'une horloge.
3. Les hooks appellent la semantique (`insn/<m>.rs`) et l'ALU pure (`alu.rs`).
4. Le prefetch (IR/IRC) est modele explicitement: c'est ce qui rend les cycles de bus exacts.
5. `step()` est construit sur `tick()` (une seule logique de timing).

## 5. Frontend

`puce-m68k-web` pilote un `Cpu` reel (via `puce-m68k-machines::session`) et pousse des instantanes JSON via WebSocket; la page (Tailwind, `docs/frontend/`) n'effectue aucun calcul CPU. Reference de conception de depart: `docs/frontend/z80_reference.html` (page du crate Z80) a adapter.
