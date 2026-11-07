from _dsl import T

C = "puce-m68k"
EA = "04-03: `docs/specs/instr/_EA.md` (timing des modes d'adressage)"

T("E05-01", "Table de decodage (65536 entrees, const fn)", kind="code", size=130,
  context="Le coeur est `no_std` sans alloc: la table opcode -> instruction est construite a la compilation par une `const fn` a partir d'une liste de regles (masque, valeur) fournie par chaque module d'instruction.",
  spec="""Cree `crates/puce-m68k/src/decode.rs` (`pub mod decode;`) et `crates/puce-m68k/src/insn/mod.rs` (`pub mod insn;`):
- `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct InsnId(pub u16);` avec `InsnId::ILLEGAL = InsnId(0)`.
- `pub struct Rule { pub mask: u16, pub value: u16, pub id: InsnId, pub variant: u8 }`.
- `pub const fn build_table(rules: &[Rule]) -> [Decoded; 65536]` ou `Decoded { id: InsnId, variant: u8 }`; une regle donne son (id, variant) a tout opcode `o` avec `o & mask == value`; deux regles qui capturent le meme opcode = `panic!` a la compilation (const), l'ordre n'est pas significatif.
- `pub static TABLE: [Decoded; 65536] = build_table(insn::RULES);` et `pub fn decode(op: u16) -> Decoded`.
- `insn/mod.rs`: `pub const RULES: &[Rule] = &[...]` (vide pour l'instant, chaque carte d'instruction ajoutera `.. mod::RULES` via une macro ou une concatenation const documentee).""",
  write=["crates/puce-m68k/src/decode.rs", "crates/puce-m68k/src/insn/mod.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("empty_illegal", "table sans regle: tout opcode vaut ILLEGAL"), ("single_rule", "regle mask=0xFFFF value=0x4E71: decode(0x4E71) = id, decode(0x4E72) = ILLEGAL"), ("mask_range", "regle mask=0xF1F8 value=0x4000 capture 8 x 8 opcodes attendus (comptage)"), ("variant", "le `variant` de la regle est restitue"), ("table_size", "la table a 65536 entrees"), ("conflict_detected", "`build_table` sur deux regles recouvrantes panique (test `#[should_panic]` en const-eval via appel runtime de la meme fonction)")])

T("E05-02", "Champs d'adressage effectif et categories de validite", kind="code", size=140,
  context="Decodage des 6 bits mode/registre d'un opcode et categories d'adressage du manuel (donnees, memoire, controle, alterable) utilisees par chaque instruction pour ses regles de decodage.",
  reads=["`docs/ref/M68000PM.pdf` chapitre 2 (modes d'adressage) si present"],
  spec="""Cree `crates/puce-m68k/src/ea.rs` (`pub mod ea;`): `pub enum Ea { DataReg(u8), AddrReg(u8), Ind(u8), PostInc(u8), PreDec(u8), Disp(u8), Index(u8), AbsW, AbsL, PcDisp, PcIndex, Imm }`, `Ea::decode(mode: u8, reg: u8) -> Option<Ea>` (None pour mode 7 reg 5..7).
Categories sous forme de fonctions `const fn is_data(mode, reg)`, `is_memory`, `is_control`, `is_alterable`, `is_data_alterable`, `is_memory_alterable`, `is_control_alterable` definies par le manuel: data = tout sauf An; memory = tout sauf Dn et An; control = (An), d16(An), d8(An,Xn), abs.W, abs.L, d16(PC), d8(PC,Xn); alterable = tout sauf d16(PC), d8(PC,Xn), #imm. Une `const fn ea_mask(f: fn(u8,u8)->bool) -> u64` renvoie un masque 64 bits indexe par `mode*8+reg` pour construire des regles.""",
  write=["crates/puce-m68k/src/ea.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("decode_all_12", "les 12 modes decodent vers la bonne variante"), ("decode_invalid", "mode 7 reg 5, 6, 7 -> None"), ("data_excludes_an", "is_data(1, n) faux pour tout n"), ("memory_excludes_dn_an", "is_memory(0,n) et (1,n) faux"), ("control_list", "control = exactement 7 combinaisons de modes (2,5,6,7/0,7/1,7/2,7/3)"), ("alterable_excludes_pc_imm", "is_alterable(7,2), (7,3), (7,4) faux"), ("data_alterable_count", "nombre de combinaisons (mode,reg) data-alterable = 50 (Dn, (An), (An)+, -(An), d16(An), d8(An,Xn): 6x8, plus abs.W et abs.L)"), ("mask_bit", "ea_mask(is_data) a le bit (0*8+3) et pas le bit (1*8+3)")])

T("E05-03", "File de prefetch (IR/IRC) et jeton Pf", kind="code", size=120,
  context="Le prefetch rend les cycles de bus exacts: IRC contient le mot suivant, IR l'instruction en cours; chaque `Pf` fait avancer la file.",
  embed=["12_PLAN_GRAMMAR.md#2. Jetons"],
  spec="""Dans le moteur (`engine/mod.rs`, `cpu.rs`): implemente l'effet du jeton `Pf`: lit le mot a l'adresse `regs.pc` avec FC = programme (superviseur `6` si SR.S, sinon utilisateur `2`), `regs.ir = regs.irc; regs.irc = mot lu; regs.pc += 2` (l'adresse est celle du prochain mot a prefetcher, cf. specs/33). Au tick de lecture de donnee du cycle, pas avant.
Ajoute `AddrSrc::Pc` dans la resolution d'adresse. Le journal inspect (E03-04) enregistre le Pf comme une lecture FC programme.
Ajoute aussi la lecture d'un mot d'extension: `Cpu::ext_word(&self) -> u16` = `regs.irc` (avant le Pf qui le consomme).""",
  write=["crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/engine/plan.rs"],
  tests=[("pf_shifts_queue", "apres un Pf: ir = ancien irc, irc = mot lu"), ("pf_advances_pc", "pc += 2"), ("pf_fc_super", "SR.S=1: FC=6"), ("pf_fc_user", "SR.S=0: FC=2"), ("pf_address", "le mot est lu a l'ancien pc"), ("pf_4_clocks", "un Pf dure 4 horloges, 6 avec 2 waits"), ("ext_word", "ext_word() = irc")],
  accept=["cargo test -p puce-m68k e05_03_", "cargo test -p puce-m68k --features inspect e05_03_"])

T("E05-04a", "Plans d'adresse effective: Dn, An, (An), (An)+, -(An)", kind="code", size=140,
  context="Fonctions qui renvoient le plan de micro-cycles pour calculer une adresse effective et lire/ecrire l'operande, modes simples. Timing: `docs/specs/instr/_EA.md` (E04-03). Si ce fichier contient UNKNOWN pour un mode, pose la question Q&R, ne devine pas.",
  reads=["`docs/specs/instr/_EA.md`", "`docs/specs/12_PLAN_GRAMMAR.md`"],
  spec="""Dans `crates/puce-m68k/src/ea.rs`: `pub fn ea_read_plan(ea: Ea, size: Size) -> Plan` et `pub fn ea_write_plan(ea: Ea, size: Size) -> Plan` (plans `&'static [Tok]`, via des `static` ou une `const fn` documentee) pour `DataReg`, `AddrReg`, `Ind`, `PostInc`, `PreDec`. `pub enum Size { Byte, Word, Long }`.
Le pre-decrement ajoute son cycle interne selon `_EA.md`; (An)+ et -(An) sur A7 en octet changent de 2 (a gerer dans le hook, pas dans le plan: documente-le).""",
  write=["crates/puce-m68k/src/ea.rs"],
  tests=[("dn_no_bus", "lecture Dn: aucun jeton de bus (duree 0)"), ("ind_word_4", "lecture (An) mot: 4 horloges"), ("ind_long_8", "lecture (An) long: 8 horloges"), ("predec_extra", "-(An) lecture: duree = valeur de _EA.md (copie du tableau)"), ("write_same_as_read_bus", "ecriture (An) mot: un seul jeton Ww"), ("postinc_hook", "(An)+: le plan contient le hook d'increment")])

T("E05-04b", "Plans d'adresse effective: d16(An), d8(An,Xn), abs, PC-relatif, immediat", kind="code", size=150,
  context="Suite de E05-04a: modes avec mot(s) d'extension (prefetch) et calcul d'index.",
  reads=["`docs/specs/instr/_EA.md`"],
  spec="""Complete `ea_read_plan`/`ea_write_plan` pour `Disp`, `Index`, `AbsW`, `AbsL`, `PcDisp`, `PcIndex`, `Imm` (immediat: taille octet/mot = 1 mot d'extension, long = 2). Chaque mot d'extension est un `Pf` (E05-03) place comme dans `_EA.md`; l'index `d8(An,Xn)` ajoute le cycle interne indique.
`ea_ext_words(ea: Ea, size: Size) -> u8` renvoie le nombre de mots d'extension (0, 1 ou 2).""",
  write=["crates/puce-m68k/src/ea.rs"],
  tests=[("disp_word_read", "d16(An) lecture mot: 1 Pf + 1 Rw (duree selon _EA.md)"), ("index_internal", "d8(An,Xn): contient un N(2)"), ("abs_w_long", "abs.W lecture long: 1 Pf + Rl"), ("abs_l_ext_words", "abs.L = 2 mots d'extension"), ("imm_long_ext_words", "#imm long = 2 mots; octet = 1"), ("pc_disp", "d16(PC) lecture mot"), ("ext_words_table", "ea_ext_words pour les 12 modes")])

T("E05-05", "Boucle d'instruction: decodage et execution (step)", kind="code", size=140,
  context="Relie decodage, plans et semantique: a la frontiere d'instruction, decoder IR, choisir le plan (insn, variante) et l'executer; `step` = executer une instruction complete.",
  spec="""Dans `cpu.rs`/`engine/mod.rs`: `fn begin_instruction(&mut self)` lit `decode::decode(self.regs.ir)`; si `ILLEGAL`, place le moteur en etat `Illegal` (journalise, aucune exception avant E11: il n'avance pas et `at_instruction_boundary()` reste vrai; documente-le). Sinon appelle `insn::plan_for(id, variant, &mut self)` qui renvoie le `Plan` (dispatch par `match id` dans `insn/mod.rs`, vide pour l'instant: renvoie un plan `&[Tok::Pf]` par defaut seulement pour `InsnId` de test).
`pub fn step<B: Bus>(&mut self, bus: &mut B) -> u32` execute jusqu'a la prochaine frontiere d'instruction et renvoie les horloges consommees (une instruction ILLEGAL renvoie 0).
Les hooks `H(k)` sont dispatches par `insn::run_hook(id, k, &mut Cpu)`.""",
  write=["crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/insn/mod.rs"],
  tests=[("illegal_no_advance", "opcode ILLEGAL: step renvoie 0"), ("test_insn_runs", "une instruction de test enregistree (feature de test) s'execute et step renvoie la duree du plan"), ("boundary", "at_instruction_boundary vrai apres step"), ("hook_dispatch", "un hook de test est appele une fois a la bonne position"), ("step_counts_waits", "avec wait states: la valeur renvoyee les inclut")])

T("E05-06", "GATE E05 - decodage, EA, prefetch, step", gate=True, kind="gate", size=0,
  context="Validation humaine avant les cartes d'instruction: decodage, adressage effectif, file de prefetch, boucle `step`.",
  user_cmds=["cargo test -p puce-m68k", "bash tools/ci.sh", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --filter NOP"], accept=[])
