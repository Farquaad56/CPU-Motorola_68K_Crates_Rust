from _dsl import T
import _insn

EPIC = "E09"
_insn.build(EPIC, "Bcc BSR DBcc Scc JMP JSR RTS RTR".split())

T("E09-99", "GATE E09 - controle de flux", gate=True, kind="gate", size=0,
  context="Validation humaine de l'epopee 'controle de flux': toutes les cartes d'instruction ont passe leur acceptance; verification globale.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E09 --strict", "bash tools/ci.sh"], accept=[])
