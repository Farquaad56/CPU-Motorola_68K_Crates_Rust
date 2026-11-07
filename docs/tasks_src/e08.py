from _dsl import T
import _insn

EPIC = "E08"
_insn.build(EPIC, "AND OR EOR NOT ASL ASR LSL LSR ROL ROR ROXL ROXR BTST BSET BCLR BCHG ABCD SBCD NBCD".split())

T("E08-99", "GATE E08 - logique, decalages, bits et BCD", gate=True, kind="gate", size=0,
  context="Validation humaine de l'epopee 'logique, decalages, bits et BCD': toutes les cartes d'instruction ont passe leur acceptance; verification globale.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E08 --strict", "bash tools/ci.sh"], accept=[])
