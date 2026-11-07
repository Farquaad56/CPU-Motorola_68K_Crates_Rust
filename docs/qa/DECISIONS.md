# DECISIONS (source de verite; jamais contredite par une tache)

| ID | Date | Decision | Origine |
|---|---|---|---|
| D-001 | 2026-11-07 | Projet = crate Rust `puce-m68k`, Motorola 68000 seul, cycle perfect, reutilisable; methode Hermes Kanban identique au crate Z80 | utilisateur |
| D-002 | 2026-11-07 | Precision micro-cycle (style FX68K/MAME). PRECISION Claude: `Cpu::tick` = 1 horloge (et non 2) car les wait states DTACK durent 1 horloge; le micro-cycle de 2 horloges reste l'unite d'ordonnancement interne | utilisateur + Claude, a confirmer |
| D-003 | 2026-11-07 | Broches modelisees: DTACK, UDS/LDS, FC0-FC2, AS, RW. Exceptions, interruptions (IPL/VPA/VMA), BERR/HALT/RESET, BR/BG/BGACK = E11, differee | utilisateur |
| D-005 | 2026-11-07 | Frontend identique au crate Z80: serveur local Rust (HTTP+WebSocket) + page Tailwind FR/EN, aucun calcul CPU en JS; demo WASM | utilisateur |
| D-006 | 2026-11-07 | Machine d'essai: RAM plate + runner SingleStepTests uniquement | utilisateur |
| D-007 | 2026-11-07 | Oracle: SingleStepTests/m68000 (genere par MAME, pas du materiel) + Yacht.txt; exceptions connues TAS et TRAPV | utilisateur |
| D-008 | 2026-11-07 | Noms: crates `puce-m68k*`, board/tenant Hermes `m68k`, profil `M68K_Crate`, binaires `m68k` et `m68k-web` | utilisateur |
| D-009 | 2026-11-07 | `puce-m68k`: entiers uniquement, deterministe, `#![forbid(unsafe_code)]`, `#![no_std]`, sans dependance | defaut (herite du Z80) |
| D-010 | 2026-11-07 | Une carte Kanban = une instruction; contexte LLM local 135000 tokens | utilisateur |
| D-011 | 2026-11-07 | Broches en logique positive (RW = signal materiel: 1 lecture) | defaut Claude |
| D-014 | 2026-11-07 | Taches de 50 a 150 lignes de code | methode Puce16 |
| D-020 | 2026-11-07 | Execution strictement sequentielle, une tache a la fois | methode Puce16 |
| D-032 | 2026-11-07 | Contraintes NeoGeo (futur projet puce-neogeo = puce-z80 + puce-m68k, voir specs/40_NEOGEO_CONSTRAINTS.md): E11-02 (traitement d'exception), E11-05 (IPL + autovecteur, niveaux 1-3) et E11-06 (RESET) ne sont plus differees; `Cpu::tick(pins)` reste disponible en plus du trait `Bus` (DTACK pilote par l'hote); aucun diviseur d'horloge dans les coeurs. SUPERSEDE partiellement D-003 | utilisateur |
| D-030 | 2026-11-07 | Les tests SingleStepTests contenant un cycle `re`/`we` (address error) sont ignores et comptes a part jusqu'a E11; `ssc --strict` ne tient pas compte des stems de `sst_known_exceptions.txt` | defaut Claude, a confirmer |
| D-031 | 2026-11-07 | Le script `decode.py` du depot SingleStepTests n'est jamais execute (code externe): le format binaire est decrit dans specs/33 et lu par du code Rust ecrit dans le projet | Claude |
| D-033 | 2026-11-07 | E03-03 peut modifier engine/mod.rs: 	ick_bus a besoin de connaitre le cycle de bus a venir (Engine::current), refactor du moteur d'E02-06 sans changement de comportement (tests e02_06_ inchanges). Scope de la carte elargi dans tasks_src/e03.py | Claude (Hermes fige, cartes realisees directement) |
| D-034 | 2026-11-07 | Le runner SST (ssc.rs) execute des instructions entieres avec Cpu::step (decodage de IR) au lieu de step_clocks; il verifie aussi le nombre total d'horloges. Modification faite avant la carte E06-01 et commitee a part (hors scope des cartes d'instruction) | Claude (Hermes fige, cartes realisees directement) |
