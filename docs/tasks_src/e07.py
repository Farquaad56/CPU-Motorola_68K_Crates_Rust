from _dsl import T
import _insn

EPIC = "E07"
_insn.build(EPIC, "ADD ADDA ADDX SUB SUBA SUBX NEG NEGX CMP CMPA TST MULU MULS DIVU DIVS".split())

T("E07-99", "GATE E07 - arithmetique entiere", gate=True, kind="gate", size=0,
  context="Validation humaine de l'epopee 'arithmetique entiere': toutes les cartes d'instruction ont passe leur acceptance; verification globale.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E07 --strict", "bash tools/ci.sh"], accept=[])
