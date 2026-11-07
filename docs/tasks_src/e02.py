from _dsl import T

T("E02-01", "Type Pins (broches du 68000)", kind="code", size=110,
  context="Representation des broches sur 64 bits, logique positive (D-011). Interface du CPU (`tick`) et des hotes.",
  embed=["11_PINS_AND_TICK.md#1. Type `Pins`"],
  spec="""Cree `crates/puce-m68k/src/pins.rs` (decommente `pub mod pins;` dans `lib.rs`): `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub struct Pins(pub u64);`
- Constantes de masque/bit documentees (`ADDR_MASK`, `DATA_SHIFT`, `AS`, `UDS`, `LDS`, `RW`, `DTACK`, `FC_SHIFT`).
- Accesseurs `addr()`, `data()`, `as_()`, `uds()`, `lds()`, `rw_read()`, `dtack()`, `fc()` et constructeurs `with_addr(u32)`, `with_data(u16)`, `with_as(bool)`, `with_uds(bool)`, `with_lds(bool)`, `with_rw_read(bool)`, `with_dtack(bool)`, `with_fc(u8)` (chaque `with_*` ne touche que ses bits).
- `with_addr` masque a 24 bits et force le bit 0 a 0.""",
  write=["crates/puce-m68k/src/pins.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("layout", "`Pins(0).with_as(true).0 == 1 << 40`, `with_rw_read(true).0 == 1 << 43`, `with_dtack(true).0 == 1 << 44`"), ("addr_mask", "`with_addr(0x0123_4567).addr() == 0x0023_4566`"), ("data_roundtrip", "`with_data(0xBEEF).data() == 0xBEEF` sans toucher aux autres bits"), ("fc_roundtrip", "`with_fc(6).fc() == 6`, `with_fc(0xFF).fc() == 7`"), ("independent", "positionner un champ ne modifie pas les autres"), ("default_zero", "`Pins::default().0 == 0`")])

T("E02-02", "Registres (Regs)", kind="code", size=120,
  context="Etat architectural du 68000: D0-D7, A0-A6, USP, SSP, SR, PC, plus IR/IRC (file de prefetch).",
  embed=["33_SINGLESTEP_FORMAT.md#3. Etat"],
  spec="""Cree `crates/puce-m68k/src/regs.rs` (`pub mod regs;`): `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub struct Regs { pub d: [u32; 8], pub a: [u32; 7], pub usp: u32, pub ssp: u32, pub sr: u16, pub pc: u32, pub ir: u16, pub irc: u16 }`.
- Constantes de SR: `C=0x01, V=0x02, Z=0x04, N=0x08, X=0x10, IMASK=0x0700, S=0x2000, T=0x8000`, masque des bits valides `SR_MASK = 0xA71F`.
- `a7(&self) -> u32` (ssp si S sinon usp), `set_a7(&mut self, v)`, `an(&self, n: u8) -> u32` (n=7 -> a7), `set_an`.
- `set_sr(&mut self, v: u16)` applique `SR_MASK`; `ccr(&self) -> u8`; `set_ccr(&mut self, v: u8)` ne change que le poids faible.
- PC et adresses: 24 bits significatifs a l'usage (pas de masquage dans le type).""",
  write=["crates/puce-m68k/src/regs.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("a7_user", "S=0: a7() = usp"), ("a7_super", "S=1: a7() = ssp"), ("set_a7", "set_a7 ecrit usp ou ssp selon S"), ("an_7", "an(7) == a7()"), ("sr_mask", "set_sr(0xFFFF) donne 0xA71F"), ("ccr", "set_ccr(0xFF) ne modifie pas l'octet haut de sr; ccr() = sr & 0x1F")])

T("E02-03", "Trait Bus de confort", kind="code", size=60,
  context="Interface simple pour les hotes qui ne veulent pas gerer les broches a la main: le coeur appelle `read`/`write` quand il execute un cycle de bus.",
  embed=["11_PINS_AND_TICK.md#4. Echantillonnage"],
  spec="""Cree `crates/puce-m68k/src/bus.rs` (`pub mod bus;`):
```rust
pub trait Bus {
    fn read(&mut self, addr: u32, fc: u8, uds: bool, lds: bool) -> u16;
    fn write(&mut self, addr: u32, fc: u8, uds: bool, lds: bool, data: u16);
    /// Extra wait clocks for the cycle that starts now (default 0 = DTACK immediate).
    fn wait_clocks(&mut self, _addr: u32, _write: bool) -> u8 { 0 }
    /// Called once per clock with the pins the CPU drives (peripheral clocks).
    fn on_tick(&mut self, _pins: crate::pins::Pins) {}
}
```
Ajoute aussi `pub enum Cycle { Fetch, Read, Write, Tas, Internal }` documente.""",
  write=["crates/puce-m68k/src/bus.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("defaults", "un bus de test qui n'implemente que read/write: `wait_clocks` renvoie 0 et `on_tick` ne plante pas"), ("object_safe", "`&mut dyn Bus` compile")])

T("E02-04", "Jetons de plan (Tok, Plan)", kind="code", size=110,
  context="Vocabulaire des plans de micro-cycles (brouillon specs/12). Cette tache valide aussi le brouillon contre Yacht.txt si `docs/ref/Yacht.txt` existe; sinon elle suit le brouillon et note les doutes en question Q&R non bloquante.",
  embed=["12_PLAN_GRAMMAR.md#2. Jetons", "12_PLAN_GRAMMAR.md#3. Regles"],
  spec="""Cree `crates/puce-m68k/src/engine/plan.rs` et `engine/mod.rs` (`pub mod engine;`): `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Tok { N(u8), Pf, Rb(AddrSrc), Rw(AddrSrc), Rl(AddrSrc), Wb(AddrSrc), Ww(AddrSrc), Wl(AddrSrc), Tas(AddrSrc), H(u8) }`, `pub enum AddrSrc { Pc, Ea, EaPlus2, An(u8), AnPre(u8), AnPost(u8), Sp, SpPre, SpPost, Vec(u8) }`, `pub type Plan = &'static [Tok];`.
`Tok::clocks(self) -> u32`: N(n) = n, Pf = 4, Rb/Rw/Wb/Ww/Tas = 4, Rl/Wl = 8, H = 0. `fn plan_clocks(p: Plan) -> u32` somme.""",
  write=["crates/puce-m68k/src/engine/plan.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("clocks_basic", "N(2)=2, Pf=4, Rw=4, Rl=8, H=0"), ("plan_sum", "plan [Pf, N(2), Rw(Ea), Pf] = 14"), ("const_plan", "un `static P: Plan = &[...]` compile")])

T("E02-05", "Profil des broches par horloge (cycle de bus)", kind="code", size=120,
  context="Fonction pure qui donne les broches de sortie a chaque horloge d'un cycle de bus de base (4 horloges).",
  embed=["11_PINS_AND_TICK.md#3. Profil"],
  spec="""Cree `crates/puce-m68k/src/engine/mcycle.rs`: `pub(crate) enum BusKind { Read, Write }` et `pub(crate) fn bus_pins(kind: BusKind, clock: u8, addr: u32, fc: u8, uds: bool, lds: bool, data: u16) -> Pins` pour `clock` 0..=3 selon la section "Profil des broches". Hors cycle de bus: `idle_pins(fc: u8, prev_addr: u32) -> Pins` (AS/UDS/LDS inactifs).
Le profil exact est marque UNKNOWN - to confirm dans la spec: implemente exactement celui du texte, et laisse un commentaire `UNKNOWN - to confirm` a chaque endroit ou le manuel pourrait differer.""",
  write=["crates/puce-m68k/src/engine/mcycle.rs", "crates/puce-m68k/src/engine/mod.rs"],
  tests=[("read_clock0", "lecture horloge 0: ADDR et FC presents, RW=1, AS=0"), ("read_clock1", "horloge 1: AS=1 et UDS/LDS selon les parametres"), ("read_clock3_release", "horloge 3: les strobes sont encore actifs (relachement a la fin du cycle)"), ("write_rw_low", "ecriture: RW=0 a partir de l'horloge 1"), ("write_data_clock2", "ecriture: DATA presente a l'horloge 2"), ("idle", "idle_pins: AS=0, UDS=0, LDS=0")])

T("E02-06", "Moteur: execution d'un plan avec attente DTACK", kind="code", size=150,
  context="Premier moteur `tick`: execute un plan de jetons en comptant les horloges et en inserant des wait states tant que DTACK=0 a l'echantillonnage. Pas de decodage d'instruction ici.",
  embed=["11_PINS_AND_TICK.md#2. Granularite du tick", "11_PINS_AND_TICK.md#4. Echantillonnage"],
  spec="""Cree `crates/puce-m68k/src/cpu.rs` (`pub mod cpu;`) et complete `engine/mod.rs`: `pub struct Cpu { pub regs: Regs, /* moteur prive */ }`, `Cpu::new()`, `fn start_plan(&mut self, p: Plan)` (pub(crate), pour les tests), `pub fn tick(&mut self, pins: Pins) -> Pins` (une horloge), `pub fn at_instruction_boundary(&self) -> bool`.
Regles: un jeton de bus dure 4 horloges (8 pour Rl/Wl = deux cycles); a l'horloge 2 de chaque cycle de bus on lit `pins.dtack()`: si faux, on repete l'horloge 2 (wait state) jusqu'a acquittement; `N(n)` dure n horloges sans bus. Les adresses des jetons sont resolues plus tard (E05): ici utilise `AddrSrc::Ea` = `regs.pc` comme adresse de test.
Comptabilite: le moteur expose `pub(crate) fn clocks(&self) -> u64` (total d'horloges ecoulees).""",
  write=["crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k/src/lib.rs"],
  tests=[("read_4_clocks", "plan [Rw(Ea)] avec DTACK=1: 4 appels de tick puis `at_instruction_boundary()`"), ("read_wait", "meme plan avec DTACK=0 pendant 3 horloges: 7 appels"), ("long_8_clocks", "plan [Rl(Ea)] = 8 horloges"), ("internal_n4", "plan [N(4)]: 4 horloges, aucune broche de bus active"), ("two_tokens", "[N(2), Rw(Ea)] = 6 horloges"), ("clock_counter", "`clocks()` compte toutes les horloges y compris les waits")])

T("E02-07", "GATE E02 - broches, registres, bus et moteur de base", gate=True, kind="gate", size=0,
  context="Validation humaine des fondations (Pins, Regs, Bus, plans, moteur).",
  user_cmds=["cargo test -p puce-m68k", "bash tools/hygiene.sh", "bash tools/ci.sh"], accept=[])
