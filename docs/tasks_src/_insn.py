"""Generator of one-card-per-instruction tasks from docs/specs/data/instructions.tsv (D-010)."""
from pathlib import Path
from _dsl import T

TSV = Path(__file__).resolve().parents[1] / "specs" / "data" / "instructions.tsv"


def _rows():
    rows = [l.split("\t") for l in TSV.read_text(encoding="utf-8").splitlines()[1:]]
    return [dict(stem=r[0], mn=r[1], size=r[2], file=r[3], epic=r[5], notes=r[6] if len(r) > 6 else "") for r in rows]


ROWS = _rows()

TEMPLATE = """Implemente `{mn}` ({variants}) dans `crates/puce-m68k/src/insn/{low}.rs` et enregistre-la dans `insn/mod.rs` (`pub mod {low};`, ses `RULES`, sa branche de `plan_for` et de `run_hook`).
Sous-taches, dans cet ordre:
1. Decodage: regles (masque, valeur) de tous les encodages LEGAUX de {mn} (modes d'adressage valides via `ea.rs`); les autres encodages ne doivent pas etre captures.
2. Plans: pour chaque (taille, mode source, mode destination) legal, un plan de micro-cycles (jetons de `docs/specs/12_PLAN_GRAMMAR.md`, `ea_read_plan`/`ea_write_plan` de `ea.rs`) conforme a `docs/specs/instr/{mn}.md`.
3. Semantique et drapeaux (X N Z V C) dans les hooks; fonctions pures partagees dans `alu.rs` si elles servent a plusieurs instructions.
4. Verification: la commande `m68k ssc ... --strict` ci-dessous doit passer (2500 tests par fichier; tests avec address error ignores, D-030).
Source de verite du timing: `docs/specs/instr/{mn}.md` puis `docs/specs/instr/_EA.md`. Jamais deviner un timing: question Q&R avec les valeurs en conflit. Les tests SingleStepTests viennent de MAME (D-007).{notes}"""


def card(epic_counter, mn, rows, split=None):
    """Create the card(s) for one mnemonic. split = [(suffix, title_extra, spec_extra, sst)] or None."""
    low = "i_" + mn.lower()  # prefix: `move` is a Rust keyword
    stems = [r["stem"] for r in rows]
    variants = ", ".join("%s" % (r["size"] if r["size"] != "-" else "sans taille") for r in rows)
    notes = ""
    for r in rows:
        if r["notes"] and r["notes"] not in notes:
            notes += "\nNote: " + r["notes"]
    spec_base = TEMPLATE.format(mn=mn, low=low, variants=variants, notes=notes)
    ctx = "Une carte = une instruction (D-010). Fichiers de test: %s." % ", ".join(r["file"] for r in rows)
    tsv = [("data/instructions.tsv", stems)]
    parts = split or [("", "", "", True)]
    for k, (suffix, title_extra, spec_extra, sst) in enumerate(parts):
        n = epic_counter[0] if k == 0 else epic_counter[0]
        tid = "%s-%02d%s" % (epic_counter[1], n, suffix)
        pre = tid.lower().replace("-", "_") + "_"
        accept = ["cargo test -p puce-m68k %s" % pre]
        if sst:
            accept += ["cargo run --release -q -p puce-m68k-cli -- ssc tests/singlestep --filter %s --strict" % s for s in stems]
        T(tid, "%s%s" % (mn, title_extra), kind="code", size=120 if split else 110, context=ctx,
          embed=[], tsv=tsv if k == 0 else [], spec=spec_base + ("\n\n" + spec_extra if spec_extra else ""),
          reads=["`docs/specs/instr/%s.md` (timing, genere en E04)" % mn, "`docs/specs/instr/_EA.md`", "`docs/specs/33_SINGLESTEP_FORMAT.md`"],
          write=["crates/puce-m68k/src/insn/%s.rs" % low, "crates/puce-m68k/src/insn/mod.rs", "crates/puce-m68k/src/alu.rs"],
          tests=[("decode_legal", "tous les encodages legaux de %s decodent vers cette instruction (comptage exact attendu: calcule-le d'apres les modes valides)" % mn),
                 ("decode_illegal", "un encodage illegal voisin n'est pas capture"),
                 ("plan_basic", "le plan du cas le plus simple a la duree donnee par docs/specs/instr/%s.md" % mn),
                 ("flags_sample", "3 cas de drapeaux calcules a la main d'apres le manuel (valeurs initiales -> X N Z V C)")],
          accept=accept)
    epic_counter[0] += 1


def build(epic, groups, start=1):
    """groups: list of mnemonics in execution order; SPLITS: mnemonic -> split list."""
    counter = [start, epic]
    for mn in groups:
        rows = [r for r in ROWS if r["mn"] == mn]
        assert rows, mn
        card(counter, mn, rows, SPLITS.get(mn))
    return counter


SPLITS = {
    "MOVE": [("a", " (1/2): sources registre et immediat", "Cette sous-tache couvre les sources `Dn`, `An` et `#imm` vers destinations `Dn` et `(An)`, en octet/mot/long. Les autres modes viennent en 2/2.", False),
             ("b", " (2/2): tous les autres modes", "Complete tous les modes source/destination legaux (y compris `(An)+`, `-(An)`, `d16(An)`, `d8(An,Xn)`, absolus, PC-relatif). Ordre des ecritures de MOVE.l en pre-decrement: voir Yacht.txt.", True)],
    "MOVEM": [("a", " (1/2): registres vers memoire", "Sens registres -> memoire (masque en mot d'extension, mode -(An) a masque inverse).", False),
              ("b", " (2/2): memoire vers registres", "Sens memoire -> registres (extension de signe des mots vers les registres, cycle de lecture supplementaire final).", True)],
    "DIVU": [("a", " (1/2): quotient et reste", "Resultat, drapeaux N Z V C pour les cas SANS depassement. Division par zero: exception, hors perimetre (E11).", False),
             ("b", " (2/2): timing de l'algorithme", "Le nombre d'horloges depend des operandes (algorithme de division): reproduis-le d'apres Yacht.txt et le code de reference de FX68K/Pasti notes dans docs/00_CADRAGE.md.", True)],
    "DIVS": [("a", " (1/2): quotient et reste signes", "Resultat, drapeaux pour les cas sans depassement; division par zero hors perimetre (E11).", False),
             ("b", " (2/2): timing de l'algorithme", "Timing dependant des operandes, voir DIVU (b).", True)],
}
