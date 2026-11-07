# 00 - CADRAGE (crate Motorola 68000 cycle perfect)

Methode heritee du crate Z80 (docs/00_METHODE.md, 01_ARCHITECTURE.md, 03_KANBAN_HERMES.md du depot "CPU Zilog Z80_Crates_Rust").

## 1. Decisions

| ID | Decision |
|---|---|
| D-001 | Perimetre: **68000 seul**. 68010+ differe (architecture sans blocage pour l'etendre). |
| D-002 | Precision **micro-cycle** (style FX68K / microcode MAME): `Cpu::tick(pins) -> pins` = 1 micro-cycle (2 horloges). Un cycle de bus = au moins 4 horloges. |
| D-003 | Broches modelisees: **DTACK, UDS/LDS, FC0-FC2, AS, R/W** (+ wait states). |
| D-004 | Hors perimetre initial (E11; SAUF E11-02, E11-05, E11-06 requis par la NeoGeo, voir D-032 et docs/specs/40_NEOGEO_CONSTRAINTS.md), a rediscuter AVANT l'epopee exceptions: address error, bus error, HALT, RESET, IPL0-2/VPA/VMA, BR/BG/BGACK. Attention: SingleStepTests/m68000 contient des cas address error. |
| D-005 | Frontend = meme systeme que le Z80: serveur local Rust (HTTP + WebSocket sur 127.0.0.1) + page embarquee FR/EN avec Tailwind, AUCUN calcul CPU en JavaScript. Demo WASM sans serveur. |
| D-006 | Machines d'essai: **RAM plate + runner SingleStepTests** uniquement (Mega Drive / Atari ST / Amiga plus tard). |
| D-007 | Oracle: **JSON SingleStepTests/m68000 + Yacht.txt**. Pas de coeur de reference supplementaire. Divergences connues (TAS, TRAPV, bugs MAME) = exceptions listees. |
| D-008 | Nommage: workspace `crates/*`: `puce-m68k`, `puce-m68k-machines`, `puce-m68k-cli`, `puce-m68k-web`, `puce-m68k-wasm`. Board/tenant Hermes: `m68k`. |
| D-009 | Coeur: `#![no_std]`, `#![forbid(unsafe_code)]`, sans alloc, zero dependance, entiers uniquement, deterministe. |
| D-010 | Kanban: **une carte = une instruction** (sous-taches: decodage, modes d'adressage, micro-timing, tests JSON). Chaine strictement sequentielle, GATE humain en fin d'epopee. |
| D-011 | Budget contexte LLM local: 135000 tokens. Chaque carte embarque ses extraits de spec (jamais de connaissance implicite) et vise 50-150 lignes de code. |

## 2. Documentation (a telecharger dans docs/ref/, jamais committee si non libre)

- Yacht.txt (timing bus au niveau microcode): http://nemesis.hacking-cult.org/MegaDrive/Documentation/Yacht.txt (discussion: https://gendev.spritesmind.net/forum/viewtopic.php?p=19775)
- M68000UM Rev 8 (tables de timing, chapitre 8): https://cdn.hackaday.io/files/1805367724052224/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf
- M68000 Programmer's Reference Manual: https://www.eecg.utoronto.ca/~moshovos/ECE243-07/M68000PM.pdf
- FX68K (recoupement manuel si doute): http://github.com/ijor/fx68k
- Timing DIVU/DIVS (Pasti): http://pasti.fxatari.com/68kdocs/

## 3. Tests

- SingleStepTests/m68000 (MIT, genere depuis le coeur microcode de MAME; .json.bin a convertir avec decode.py): https://github.com/SingleStepTests/m68000
  - RAM en mots 16 bits, UDS/LDS dans les transactions, PC = prochain prefetch (+4), cycles `re`/`we` pour address errors.
  - Connus douteux: TAS (timing special RMW non modelise), TRAPV.
- ROM decodage "opcode sizes" (passe sur materiel): https://github.com/realmonster/smd_emu_tests
- Aucune ROM de timing verifiee sur materiel n'existe publiquement (a notre connaissance).
- Non verifies: testeur CPU WinUAE, suite de tests de Moira, depot archive SingleStepTests/ProcessorTests.

## 4. Epopees (generees dans docs/tasks_src/, voir docs/tasks/INDEX.tsv)

- E01 Squelette workspace, CI, hygiene (fait par Claude, GATE a approuver)
- E02 Pins, Regs, Bus, jetons de plan, moteur de base (tick = 1 horloge)
- E03 Runner SingleStepTests (lecteur .json.bin, RAM plate, journal de bus, commande `m68k ssc`)
- E04 Extraction des specs par instruction depuis Yacht.txt et le manuel (GATE: `fetch_refs.py --only docs`)
- E05 Table de decodage, modes d'adressage, prefetch, boucle `step`
- E06..E10 Une carte par instruction (127 fichiers de tests): transferts, arithmetique, logique/decalages/bits/BCD, controle de flux, registres d'etat
- E11 Exceptions et interruptions (DIFFEREE, D-003): address error, trap, privilege, IPL, RESET/HALT
- E12 Desassembleur et CLI (run, disasm, fetch)
- E13 Serveur web + page Tailwind FR/EN (HTTP + WebSocket)
- E14 Demo WebAssembly
- E15 Performance, baseline complete SingleStepTests, documentation, GATE final

## 5. Questions ouvertes

- Contenu exact de la page web (registres D0-D7/A0-A7, SR, desassembleur, memoire, points d'arret, journal de bus?)
- Moment de l'epopee exceptions (D-004)
- Modele Hermes local utilise et sa limite pratique de contexte
