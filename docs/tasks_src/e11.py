from _dsl import T
import _insn

DEF = "DIFFEREE (D-003): hors perimetre initial; l'utilisateur decide au GATE E11-01 si l'epopee est lancee."

T("E11-01", "GATE E11 - demarrage des exceptions et interruptions", gate=True, kind="gate", size=0, deferred=True,
  context="E11-02, E11-05 et E11-06 sont REQUISES par le futur projet NeoGeo (D-032); E11-03, E11-04 et les cartes d'instruction E11-07+ restent differees (D-003). Approuve ce gate pour lancer E11-02. Rappel: tous les tests SingleStepTests contenant une address error (cycles `re`/`we`) sont ignores jusqu'ici. Cette epopee les active.",
  user_cmds=["cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E10 --strict"], accept=[])
T("E11-02", "Traitement d'exception (groupes 1 et 2) et vecteurs", kind="code", size=150, deferred=True,
  context=DEF + " Sequence commune: copie de SR, passage en superviseur, empilement PC et SR, lecture du vecteur, prefetch du handler. Timing: Yacht.txt (exceptions, points signales douteux).",
  reads=["`docs/specs/instr/` (exceptions) si extraites"], spec="Cree `crates/puce-m68k/src/exception.rs`: plan generique `exception_plan(vector: u8)`, empilement 6 octets, lecture du vecteur FC=donnees superviseur, prefetch. Aucune dependance a une instruction particuliere.",
  write=["crates/puce-m68k/src/exception.rs", "crates/puce-m68k/src/lib.rs", "crates/puce-m68k/src/cpu.rs"],
  tests=[("frame_layout", "pile: SR a SP, PC haut a SP+2, PC bas a SP+4"), ("super_mode", "S=1, T=0 apres l'exception"), ("vector_address", "vecteur n lu a 4*n"), ("clocks", "duree totale = valeur de Yacht.txt pour le vecteur 4")])
T("E11-03", "Address error (frame de groupe 0)", kind="code", size=150, deferred=True,
  context=DEF + " Frame de 14 octets (mot de statut, adresse fautive, IR, SR, PC). Active les tests `re`/`we`: retire le skip D-030 du runner pour les fichiers couverts.",
  spec="Detecte l'acces mot/long a une adresse impaire dans le moteur, produit les cycles `re`/`we` (AS non asserte, resultat non commite), puis le traitement d'exception (vecteur 3). Met a jour `ssc::run_test` pour comparer ces tests au lieu de les ignorer.",
  write=["crates/puce-m68k/src/exception.rs", "crates/puce-m68k/src/engine/mod.rs", "crates/puce-m68k-machines/src/ssc.rs"],
  tests=[("odd_word_read", "lecture mot en adresse impaire declenche le vecteur 3"), ("frame_14_bytes", "14 octets empiles"), ("status_word", "mot de statut: FC et bit R/W corrects"), ("no_commit", "l'ecriture fautive n'est pas commitee")])
T("E11-04", "Violation de privilege, opcode illegal, lignes A et F", kind="code", size=110, deferred=True,
  context=DEF, spec="Branche les vecteurs 4 (illegal), 8 (privilege), 10 (ligne A), 11 (ligne F) dans `begin_instruction` (remplace le comportement `Illegal` de E05-05) et ajoute le test de privilege aux instructions privilegiees.",
  write=["crates/puce-m68k/src/exception.rs", "crates/puce-m68k/src/cpu.rs", "crates/puce-m68k/src/insn/mod.rs"],
  tests=[("illegal_vector4", "opcode non decode -> vecteur 4"), ("line_a_vector10", "0xA000 -> vecteur 10"), ("line_f_vector11", "0xF000 -> vecteur 11"), ("privilege_vector8", "instruction privilegiee en mode utilisateur -> vecteur 8")])
T("E11-05", "Interruptions IPL0-2 et autovecteur", kind="code", size=140, deferred=True,
  context=DEF, embed=["40_NEOGEO_CONSTRAINTS.md#1. Interruptions"],
  spec="Ajoute les broches IPL0-2 (bits 48..50) et VPA (bit 51) a `Pins` (reserves dans specs/11), la comparaison avec SR.IMASK a la frontiere d'instruction, le cycle d'acquittement (FC=7) et l'autovecteur (VPA) ou le vecteur fourni.",
  write=["crates/puce-m68k/src/pins.rs", "crates/puce-m68k/src/exception.rs", "crates/puce-m68k/src/cpu.rs"],
  tests=[("neogeo_levels", "niveaux 1, 2 et 3 en autovecteur: vecteurs 25, 26, 27"), ("host_vector", "un hote peut fournir un numero de vecteur au lieu de VPA"), ("level_masked", "IPL <= IMASK ignore"), ("level_taken", "IPL > IMASK pris, IMASK = niveau"), ("level7_nmi", "niveau 7 toujours pris"), ("ack_fc7", "cycle d'acquittement avec FC=7"), ("autovector", "vecteur 24 + niveau")])
T("E11-06", "RESET, HALT et double faute de bus", kind="code", size=110, deferred=True,
  context=DEF, spec="Broches RESET/HALT: sequence de reset (SSP, PC depuis les vecteurs 0 et 1), etat halted apres une double faute.",
  write=["crates/puce-m68k/src/pins.rs", "crates/puce-m68k/src/exception.rs", "crates/puce-m68k/src/cpu.rs"],
  tests=[("reset_vectors", "SSP et PC pris aux adresses 0 et 4"), ("reset_state", "S=1, IMASK=7, T=0"), ("double_fault_halt", "address error dans un traitement d'exception: halted")])
_insn.build("E11", "TRAP TRAPV CHK ILLEGAL_LINEA ILLEGAL_LINEF RTE STOP RESET".split(), start=7)
import _dsl
NEOGEO_REQUIRED = ("E11-02", "E11-05", "E11-06")  # D-032: needed by the future NeoGeo project
for _t in _dsl.TASKS:
    if _t["id"].startswith("E11"):
        _t["deferred"] = _t["id"] not in NEOGEO_REQUIRED
        if _t["id"] in NEOGEO_REQUIRED:
            _t["context"] = _t["context"].replace(DEF, "REQUIS par le futur projet NeoGeo (D-032, docs/specs/40_NEOGEO_CONSTRAINTS.md).")
