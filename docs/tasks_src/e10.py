from _dsl import T
import _insn

EPIC = "E10"
_insn.build(EPIC, "ANDItoCCR ANDItoSR ORItoCCR ORItoSR EORItoCCR EORItoSR MOVEtoCCR MOVEtoSR MOVEfromSR MOVEtoUSP MOVEfromUSP TAS".split())

T("E10-99", "GATE E10 - registres d'etat et divers", gate=True, kind="gate", size=0,
  context="Validation humaine de l'epopee 'registres d'etat et divers': toutes les cartes d'instruction ont passe leur acceptance; verification globale.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E10 --strict", "bash tools/ci.sh"], accept=[])
