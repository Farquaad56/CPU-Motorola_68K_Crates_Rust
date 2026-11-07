from _dsl import T
import _insn

EPIC = "E06"
_insn.build(EPIC, "NOP MOVE MOVEA MOVEQ MOVEM MOVEP LEA PEA EXG SWAP EXT LINK UNLINK CLR".split())

T("E06-99", "GATE E06 - transferts de donnees", gate=True, kind="gate", size=0,
  context="Validation humaine de l'epopee 'transferts de donnees': toutes les cartes d'instruction ont passe leur acceptance; verification globale.",
  user_cmds=["cargo test --workspace", "cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --epic E06 --strict", "bash tools/ci.sh"], accept=[])
