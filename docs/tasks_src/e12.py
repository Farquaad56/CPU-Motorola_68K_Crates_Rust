from _dsl import T

C = "puce-m68k"
DIS = "Desassembleur sans allocation (feature `disasm`): `pub fn disasm(words: &[u16], pc: u32, out: &mut dyn core::fmt::Write) -> usize` renvoie le nombre de mots consommes; syntaxe Motorola (`move.w d0,(a1)+`, `dc.w $xxxx` pour un opcode illegal)."

T("E12-01", "Desassembleur: infrastructure et formatage des operandes", kind="code", size=140,
  context=DIS, spec="Cree `crates/puce-m68k/src/disasm.rs` (`#[cfg(feature = \"disasm\")] pub mod disasm;`): table de mnemoniques par `InsnId`, formatage des 12 modes d'adressage (`d0`, `a1`, `(a2)`, `(a2)+`, `-(a2)`, `$10(a2)`, `$10(a2,d3.w)`, `$1234.w`, `$123456.l`, `$10(pc)`, `$10(pc,d3.l)`, `#$12`), suffixes de taille, liste de registres de MOVEM (`d0-d3/a0/a2`).",
  write=["crates/puce-m68k/src/disasm.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("ea_all_modes", "les 12 modes formates comme dans la spec"), ("size_suffix", ".b .w .l"), ("movem_list", "masque 0x0F0F -> `d0-d3/a0-a3` ou forme equivalente documentee"), ("illegal_dcw", "opcode illegal -> `dc.w $xxxx`, longueur 1"), ("no_alloc", "le module n'utilise ni `String` ni `Vec` (grep du source)")],
  accept=["cargo test -p puce-m68k --features disasm e12_01_"])
T("E12-02", "Desassembleur: transferts et arithmetique", kind="code", size=140, context=DIS,
  spec="Textes de MOVE/MOVEA/MOVEQ/MOVEM/MOVEP/LEA/PEA/EXG/SWAP/EXT/LINK/UNLK/CLR, ADD/ADDA/ADDX/SUB/SUBA/SUBX/NEG/NEGX/CMP/CMPA/TST/MUL/DIV (et les formes immediates ADDI/ADDQ/SUBI/SUBQ/CMPI).",
  write=["crates/puce-m68k/src/disasm.rs"],
  tests=[("move_w", "0x3011 -> `move.w (a1),d0`"), ("moveq", "0x7001 -> `moveq #1,d0`"), ("lea", "0x41F9 + abs.l"), ("addq", "0x5240 -> `addq.w #1,d0`"), ("movem_pre", "movem.l d0-d7,-(sp)"), ("mulu_divs", "mulu.w, divs.w")],
  accept=["cargo test -p puce-m68k --features disasm e12_02_"])
T("E12-03", "Desassembleur: logique, decalages, bits, BCD", kind="code", size=140, context=DIS,
  spec="Textes de AND/OR/EOR/NOT et formes immediates, ASL..ROXR (registre et memoire), BTST/BSET/BCLR/BCHG, ABCD/SBCD/NBCD.",
  write=["crates/puce-m68k/src/disasm.rs"],
  tests=[("and_w", "and.w d1,d0"), ("lsl_imm", "lsl.w #2,d0"), ("asr_mem", "asr.w (a0)"), ("btst_imm", "btst #3,d0"), ("abcd", "abcd d1,d0 et abcd -(a1),-(a0)")],
  accept=["cargo test -p puce-m68k --features disasm e12_03_"])
T("E12-04", "Desassembleur: controle de flux et systeme", kind="code", size=140, context=DIS,
  spec="Textes de Bcc/BSR/DBcc/Scc/JMP/JSR/RTS/RTR/RTE/NOP/STOP/RESET/TRAP/TRAPV/CHK, MOVE to/from SR/CCR/USP, ANDI/ORI/EORI to SR/CCR, TAS, ILLEGAL. Les cibles de branchement sont affichees en adresse absolue calculee depuis `pc`.",
  write=["crates/puce-m68k/src/disasm.rs"],
  tests=[("beq", "beq.s avec deplacement 8 bits -> cible absolue"), ("bra_w", "bra.w avec mot d'extension"), ("dbf", "dbf d0,target"), ("jsr_abs", "jsr $1234.l"), ("rts_nop", "rts / nop"), ("trap", "trap #15"), ("move_sr", "move sr,d0 / move d0,sr")],
  accept=["cargo test -p puce-m68k --features disasm e12_04_"])
T("E12-05", "Desassembleur: couverture des 65536 opcodes", kind="code", size=90, crate="puce-m68k-machines",
  context="Garde-fou: le desassembleur et la table de decodage doivent s'accorder pour tous les opcodes.",
  spec="Test d'integration `crates/puce-m68k-machines/tests/disasm_all.rs`: pour chaque opcode 0..=65535 (avec des mots d'extension de test), `disasm` ne panique pas; un opcode que `decode` classe ILLEGAL donne `dc.w`, un opcode decode donne un mnemonique commencant par celui de la table.",
  write=["crates/puce-m68k-machines/tests/disasm_all.rs"],
  tests=[("no_panic", "65536 opcodes sans panique"), ("illegal_matches_decode", "dc.w exactement pour les ILLEGAL"), ("length_matches", "la longueur desassemblee = 1 + nombre de mots d'extension de ea.rs")],
  accept=["cargo test -p puce-m68k-machines e12_05_"])
T("E12-06", "CLI: commandes run et disasm", kind="code", size=130, crate="puce-m68k-cli",
  context="Commandes utilisateur sur la RAM plate.",
  spec="`m68k run <bin> [--max-instr N] [--org HEX]`: charge le binaire (par defaut en 0, SSP/PC lus aux vecteurs 0 et 1, sinon `--org` et PC=org), execute jusqu'a `STOP`/illegal/`max-instr`, affiche D0-D7 A0-A7 SR PC et le nombre d'horloges. `m68k disasm <bin> --org HEX [--count N]`: une ligne `addr: octets  texte` par instruction.",
  write=["crates/puce-m68k-cli/src/main.rs", "crates/puce-m68k-cli/src/commands.rs"],
  tests=[("run_nop_loop", "programme `nop; bra.s -4` avec --max-instr 10: 10 instructions"), ("disasm_lines", "3 instructions donnent 3 lignes"), ("bad_file", "fichier absent -> Err clair"), ("org_parse", "`--org 0x1000` et `--org 1000` acceptes")])
T("E12-07", "CLI: commande fetch", kind="code", size=40, crate="puce-m68k-cli",
  context="Raccourci vers `python docs/tools/fetch_refs.py`.",
  spec="`m68k fetch [--only docs|tests]` lance `python docs/tools/fetch_refs.py` (essaie `python3` si `python` est absent) et propage le code de sortie.",
  write=["crates/puce-m68k-cli/src/commands.rs", "crates/puce-m68k-cli/src/main.rs"],
  tests=[("parse_only", "`fetch --only docs` se parse"), ("cmd_args", "la fonction `fetch_args(only)` construit les arguments du sous-processus")])
T("E12-08", "GATE E12 - desassembleur et CLI", gate=True, kind="gate", size=0,
  context="Validation humaine du desassembleur et de la CLI.",
  user_cmds=["cargo test --workspace --all-features", "cargo run -q -p puce-m68k-cli -- disasm tests/roms/<un binaire> --org 0 --count 20"], accept=[])
