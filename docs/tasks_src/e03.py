from _dsl import T

M = "puce-m68k-machines"

T("E03-01", "Lecteur du format SingleStepTests (.json.bin)", kind="code", size=150, crate=M,
  context="Le runner est la premiere chose a construire: chaque carte d'instruction a besoin de `m68k ssc` pour son acceptance. Cette tache lit le format binaire (D-031: le `decode.py` du depot n'est PAS execute).",
  embed=["33_SINGLESTEP_FORMAT.md#1. Fichier", "33_SINGLESTEP_FORMAT.md#2. Un test", "33_SINGLESTEP_FORMAT.md#3. Etat", "33_SINGLESTEP_FORMAT.md#4. Transactions"],
  spec="""Dans `crates/puce-m68k-machines/src/ssc.rs`: `pub struct State { pub d: [u32; 8], pub a: [u32; 7], pub usp: u32, pub ssp: u32, pub sr: u32, pub pc: u32, pub prefetch: [u32; 2], pub ram: Vec<(u32, u16)> }`,
`pub enum Kind { Idle, Write, Read, Tas, ReadAddrErr, WriteAddrErr }`, `pub struct Trans { pub kind: Kind, pub cycles: u32, pub fc: u32, pub addr: u32, pub data: u32, pub uds: u32, pub lds: u32 }` (champs fc..lds a 0 pour Idle),
`pub struct Test { pub name: String, pub initial: State, pub final_: State, pub trans: Vec<Trans>, pub length: u32 }`,
`pub fn parse_file(bytes: &[u8]) -> Result<Vec<Test>, String>` (erreur claire si magic faux ou fichier tronque; jamais de panic).
Ecris aussi dans le module de test un encodeur minimal d'un fichier a 1 test pour tester le lecteur sans fichier reel.
Les deux points UNKNOWN de la spec (addr_bus octet/mot, cycles minimal) sont leves ICI: lis les 5 premiers tests de `tests/singlestep/NOP.json.bin` et `MOVE.w.json.bin`, ajoute la reponse (avec 2 exemples chiffres) dans `docs/specs/33_SINGLESTEP_FORMAT.md` section 4 (remplace les lignes UNKNOWN).""",
  write=["crates/puce-m68k-machines/src/ssc.rs", "docs/specs/33_SINGLESTEP_FORMAT.md"],
  tests=[("roundtrip", "fichier synthetique a 1 test: tous les champs relus a l'identique"), ("bad_magic", "magic de fichier faux -> Err"), ("truncated", "fichier tronque -> Err, pas de panic"), ("kinds", "les 6 valeurs de `kind` (0..5) sont decodees; 6 -> Err"), ("ram_words", "une entree RAM (addr pair, mot) est conservee telle quelle"), ("real_nop", "si `tests/singlestep/NOP.json.bin` existe: 2500 tests, chacun avec `length` > 0 (sinon le test passe sans rien verifier)")])

T("E03-02", "Machine RAM plate (FlatRam)", kind="code", size=100, crate=M,
  context="RAM 16 Mio pour le runner et les futures machines. Remise a zero rapide entre deux tests: on ne reinitialise que les mots touches.",
  spec="""Dans `crates/puce-m68k-machines/src/flat.rs`: `pub struct FlatRam { mem: Box<[u8]> /* 16 MiB */, touched: Vec<u32> }`, `FlatRam::new()`,
`impl puce_m68k::bus::Bus for FlatRam` (adresse masquee a 24 bits; `read` renvoie le mot a `addr & !1` en tenant compte de `uds`/`lds`: octet non selectionne = 0; `write` n'ecrit que les octets selectionnes et memorise l'adresse du mot dans `touched`),
`poke_word(addr, w)` / `peek_word(addr)` / `peek_byte` (sans journal), `clear_touched(&mut self)` (remet a zero tous les mots touches puis vide la liste).
Les acces sont toujours alignes sur le mot dans cette machine (l'octet 'haut' est a l'adresse paire).""",
  write=["crates/puce-m68k-machines/src/flat.rs"],
  tests=[("word_rw", "ecriture mot 0x1234 en 0x1000 puis lecture avec uds+lds = 0x1234"), ("byte_high", "ecriture avec uds seul de 0xAB00: octet a 0x1000 = 0xAB, 0x1001 inchange"), ("byte_low", "lds seul: octet a 0x1001"), ("read_selects", "lecture avec uds seul renvoie 0xAB00"), ("addr_mask", "0xFF00_1000 equivaut a 0x00_1000"), ("clear_touched", "apres clear_touched la memoire ecrite vaut 0")])

T("E03-03", "Etat du CPU: load_state, state, tick_bus", kind="code", size=130,
  context="Le runner doit charger l'etat initial d'un test, executer N horloges et relire l'etat. `tick_bus` branche `tick` (broches) sur un `Bus`.",
  embed=["33_SINGLESTEP_FORMAT.md#3. Etat"],
  spec="""Dans `crates/puce-m68k/src/state.rs` (`pub mod state;`): `pub struct CpuState { pub d: [u32; 8], pub a: [u32; 7], pub usp: u32, pub ssp: u32, pub sr: u16, pub pc: u32, pub prefetch: [u16; 2] }`.
Dans `cpu.rs`: `Cpu::load_state(&mut self, s: &CpuState)` (IR = prefetch[0], IRC = prefetch[1], le moteur est place a une frontiere d'instruction) et `Cpu::state(&self) -> CpuState`.
`Cpu::tick_bus<B: Bus>(&mut self, bus: &mut B)`: construit les broches d'entree (DTACK=1 sauf pendant les `bus.wait_clocks` du cycle courant; `bus.read` appele a l'horloge ou le CPU echantillonne les donnees, `bus.write` a l'horloge ou la donnee est valide), appelle `tick`, puis `bus.on_tick(out)`.
`Cpu::step_clocks<B: Bus>(&mut self, bus: &mut B, n: u32)` = n appels de `tick_bus`.""",
  write=["crates/puce-m68k/src/state.rs", "crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("load_roundtrip", "load_state puis state() donne le meme CpuState"), ("ir_irc", "prefetch[0] -> regs.ir et prefetch[1] -> regs.irc"), ("boundary_after_load", "`at_instruction_boundary()` vrai apres load_state"), ("tick_bus_read", "plan [Rw(Ea)] sur un bus de test: `read` appele exactement une fois avec fc et uds/lds corrects"), ("tick_bus_wait", "`wait_clocks` renvoie 2: le cycle dure 6 horloges"), ("step_clocks", "step_clocks(4) = 4 ticks")])

T("E03-04", "Journal de bus (feature inspect)", kind="code", size=100,
  context="Le runner compare les transactions de bus: le coeur doit les enregistrer quand la feature `inspect` est active, sans changer son comportement et sans allocation.",
  embed=["33_SINGLESTEP_FORMAT.md#4. Transactions"],
  spec="""Dans `crates/puce-m68k/src/probe.rs` (`#[cfg(feature = "inspect")] pub mod probe;`): `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub struct BusEvent { pub kind: u8 /*0 idle,1 write,2 read,3 tas,4 re,5 we*/, pub clocks: u32, pub fc: u8, pub addr: u32, pub data: u16, pub uds: bool, pub lds: bool }`,
journal a taille fixe (`[BusEvent; 64]` + longueur) dans `Cpu`; `Cpu::journal(&self) -> &[BusEvent]`, `Cpu::clear_journal(&mut self)`.
Chaque cycle de bus execute (jeton R*/W*/Tas) ajoute un evenement; les jetons `N(n)` consecutifs fusionnent en un evenement idle. `clocks` inclut les wait states. Debordement: les evenements en trop sont comptes dans `Cpu::journal_overflow()`, pas stockes.""",
  write=["crates/puce-m68k/src/probe.rs", "crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("read_event", "un cycle de lecture produit 1 evenement kind=2, clocks=4, adresse/fc/uds/lds corrects"), ("wait_included", "avec 2 horloges d'attente: clocks=6"), ("idle_merge", "N(2) puis N(4) = 1 evenement idle de 6"), ("overflow", "au-dela de 64 evenements le compteur de debordement augmente"), ("clear", "clear_journal vide la liste"), ("no_inspect_build", "le crate compile sans la feature (test de compilation par cargo build)")],
  accept=["cargo build -p puce-m68k", "cargo test -p puce-m68k --features inspect e03_04_"])

T("E03-05", "Runner: execution et comparaison d'un test", kind="code", size=150, crate=M,
  context="Execute un test SingleStepTests: charge l'etat, joue `length` horloges, compare etat final et transactions.",
  embed=["33_SINGLESTEP_FORMAT.md#5. Comparaison attendue", "30_TEST_SUITES.md#Regles de lecture"],
  spec="""Dans `ssc.rs`: `pub enum Outcome { Pass, SkipAddrError, Fail(String) }`, `pub fn run_test(cpu: &mut Cpu, ram: &mut FlatRam, t: &Test) -> Outcome`.
1. Si une transaction du test est `ReadAddrErr`/`WriteAddrErr` -> `SkipAddrError` (D-030) sans rien executer.
2. `ram.clear_touched()`, ecrit `initial.ram` (mots), `cpu.load_state(...)` (sr, pc, prefetch...), vide le journal, `cpu.step_clocks(&mut ram, t.length)`.
3. Compare: registres et SR de `final_`, file de prefetch, memoire (chaque mot de `final_.ram`), puis journal vs `t.trans` (kind, cycles, fc, addr, data, uds, lds; octet seul -> donnee sur le bus comme dans la spec 33).
4. `Fail(msg)`: le message nomme le test, le premier ecart (champ, attendu, obtenu) et le numero de transaction.""",
  write=["crates/puce-m68k-machines/src/ssc.rs"],
  tests=[("skip_addr_error", "un test avec un cycle `re` donne SkipAddrError"), ("fail_reg", "un test dont d0 final est faux donne Fail avec le nom du registre"), ("fail_cycles", "une transaction de duree differente donne Fail avec l'indice"), ("pass_synthetic", "un test synthetique construit pour NOP et un coeur factice passe"), ("msg_names_test", "le message contient le nom du test")])

T("E03-06", "Runner: fichiers, repertoire, exceptions connues", kind="code", size=140, crate=M,
  context="Execute un fichier ou tout un repertoire, applique le filtre (stem exact ou epopee) et la liste d'exceptions connues, produit un resume.",
  reads=["`docs/specs/data/instructions.tsv` (colonnes stem, epic) et `docs/specs/data/sst_known_exceptions.txt`"],
  spec="""Dans `ssc.rs`: `pub struct FileSummary { pub stem: String, pub total: u32, pub passed: u32, pub failed: u32, pub skipped_addr_error: u32, pub first_failures: Vec<String> /* 5 max */ }`,
`pub fn run_file(path: &Path, cpu: &mut Cpu, ram: &mut FlatRam) -> Result<FileSummary, String>`,
`pub struct Selection { pub stems: Vec<String> /* vide = tous */ }`, `Selection::from_filters(&[String])` (un filtre = stem exact `ADD.b`, ou nom sans suffixe qui selectionne `NOM` et `NOM.*`), `Selection::from_epic(tsv: &Path, epic: &str)`,
`pub fn run_dir(dir: &Path, sel: &Selection) -> Result<Vec<FileSummary>, String>` (ordre alphabetique, erreur si un stem demande n'existe pas),
`pub fn load_known(path: &Path) -> Result<Vec<String>, String>` (premiere colonne tabulee, lignes vides ignorees),
`pub fn is_clean(s: &FileSummary, known: &[String]) -> bool` (vrai si `failed == 0` ou si le stem de base (avant le premier point) est dans `known`).""",
  write=["crates/puce-m68k-machines/src/ssc.rs"],
  tests=[("filter_exact", "filtre `ADD.b` ne selectionne que ADD.b"), ("filter_base", "filtre `ADD` selectionne ADD.b ADD.l ADD.w mais pas ADDA ni ADDX"), ("epic_select", "from_epic sur un tsv de test selectionne les bons stems"), ("known_parse", "load_known lit TAS et TRAPV du vrai fichier"), ("is_clean_known", "TAS avec 5 echecs est propre; ADD.b avec 1 echec ne l'est pas"), ("missing_stem", "stem inexistant -> Err")])

T("E03-07", "Commande m68k ssc", kind="code", size=110, crate="puce-m68k-cli",
  context="Interface de ligne de commande du runner, utilisee par l'acceptance de CHAQUE carte d'instruction.",
  spec="""Dans `crates/puce-m68k-cli/src/main.rs` et `commands.rs`: `m68k ssc <dir> [--filter F]... [--epic E06] [--strict] [--max-fail N] [--known FILE]` (`--filter` repetable; `--known` defaut `docs/specs/data/sst_known_exceptions.txt`; `--epic` lit `docs/specs/data/instructions.tsv`).
Sortie: une ligne par fichier `STEM total=2500 pass=.. fail=.. skip_addr=..`, les `first_failures` indentees sous les fichiers en echec, puis `TOTAL total=.. pass=.. fail=.. skip_addr=..`.
Code de sortie: 0 si pas de `--strict`; avec `--strict`: 1 si un fichier n'est pas `is_clean` (les echecs des stems connus ne comptent pas), sinon 0. `--max-fail N` arrete a N echecs au total.
Remplace le stub `ssc` (E01-03).""",
  write=["crates/puce-m68k-cli/src/main.rs", "crates/puce-m68k-cli/src/commands.rs"],
  tests=[("parse_ssc", "`m68k ssc tests/singlestep --filter NOP --strict` se parse"), ("parse_epic", "`--epic E06` accepte"), ("parse_filters", "deux `--filter` donnent deux filtres"), ("bad_flag", "flag inconnu -> erreur"), ("strict_exit", "fonction `exit_code(summaries, known, strict)`: 1 si un fichier non connu echoue, 0 si seul TAS echoue")])

T("E03-08", "GATE E03 - runner valide (tous les tests echouent: attendu)", gate=True, kind="gate", size=0,
  context="Le CPU n'execute encore aucune instruction: le runner doit donc rapporter des ECHECS clairs, pas de panic. C'est la preuve que le banc de mesure fonctionne avant d'implementer les instructions.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --filter NOP", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --filter NOP --strict ; echo exit=$?"],
  accept=[])
