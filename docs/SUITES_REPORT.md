# Rapport des suites SingleStepTests (m68000)

Genere par `m68k ssc tests/singlestep --json tests/baseline.json` (carte E15-02).

## Resume

- Fichiers: 127 (2500 cas chacun)
- Cas: 317500, reussis: 317500, echoues: 0, ignores (erreur d adresse): 0
- Cas executes: 317500, dont 317500 reussis (100.00 %)

## Echecs

Aucun: tous les cas executes passent. `TAS` est compte comme reussi depuis D-051 (le banc compare la lecture du cycle TAS comme une lecture simple, comme l oracle). La liste `docs/specs/data/sst_known_exceptions.txt` est vide.

## Cas ignores

Les cas d erreur d adresse sont executes et compares comme les autres (E11-03, D-052); `skip_addr` reste a 0.

## Reference et regression

`tests/baseline.json` fixe le resultat attendu par fichier. `m68k ssc tests/singlestep --strict --compare tests/baseline.json` retourne le code 1 si un fichier perd des cas reussis ou disparait.

## Detail par fichier

| Fichier | Total | Reussis | Echecs | Ignores |
|---|---:|---:|---:|---:|
| ABCD | 2500 | 2500 | 0 | 0 |
| ADD.b | 2500 | 2500 | 0 | 0 |
| ADD.l | 2500 | 2500 | 0 | 0 |
| ADD.w | 2500 | 2500 | 0 | 0 |
| ADDA.l | 2500 | 2500 | 0 | 0 |
| ADDA.w | 2500 | 2500 | 0 | 0 |
| ADDX.b | 2500 | 2500 | 0 | 0 |
| ADDX.l | 2500 | 2500 | 0 | 0 |
| ADDX.w | 2500 | 2500 | 0 | 0 |
| AND.b | 2500 | 2500 | 0 | 0 |
| AND.l | 2500 | 2500 | 0 | 0 |
| AND.w | 2500 | 2500 | 0 | 0 |
| ANDItoCCR | 2500 | 2500 | 0 | 0 |
| ANDItoSR | 2500 | 2500 | 0 | 0 |
| ASL.b | 2500 | 2500 | 0 | 0 |
| ASL.l | 2500 | 2500 | 0 | 0 |
| ASL.w | 2500 | 2500 | 0 | 0 |
| ASR.b | 2500 | 2500 | 0 | 0 |
| ASR.l | 2500 | 2500 | 0 | 0 |
| ASR.w | 2500 | 2500 | 0 | 0 |
| BCHG | 2500 | 2500 | 0 | 0 |
| BCLR | 2500 | 2500 | 0 | 0 |
| BSET | 2500 | 2500 | 0 | 0 |
| BSR | 2500 | 2500 | 0 | 0 |
| BTST | 2500 | 2500 | 0 | 0 |
| Bcc | 2500 | 2500 | 0 | 0 |
| CHK | 2500 | 2500 | 0 | 0 |
| CLR.b | 2500 | 2500 | 0 | 0 |
| CLR.l | 2500 | 2500 | 0 | 0 |
| CLR.w | 2500 | 2500 | 0 | 0 |
| CMP.b | 2500 | 2500 | 0 | 0 |
| CMP.l | 2500 | 2500 | 0 | 0 |
| CMP.w | 2500 | 2500 | 0 | 0 |
| CMPA.l | 2500 | 2500 | 0 | 0 |
| CMPA.w | 2500 | 2500 | 0 | 0 |
| DBcc | 2500 | 2500 | 0 | 0 |
| DIVS | 2500 | 2500 | 0 | 0 |
| DIVU | 2500 | 2500 | 0 | 0 |
| EOR.b | 2500 | 2500 | 0 | 0 |
| EOR.l | 2500 | 2500 | 0 | 0 |
| EOR.w | 2500 | 2500 | 0 | 0 |
| EORItoCCR | 2500 | 2500 | 0 | 0 |
| EORItoSR | 2500 | 2500 | 0 | 0 |
| EXG | 2500 | 2500 | 0 | 0 |
| EXT.l | 2500 | 2500 | 0 | 0 |
| EXT.w | 2500 | 2500 | 0 | 0 |
| ILLEGAL_LINEA | 2500 | 2500 | 0 | 0 |
| ILLEGAL_LINEF | 2500 | 2500 | 0 | 0 |
| JMP | 2500 | 2500 | 0 | 0 |
| JSR | 2500 | 2500 | 0 | 0 |
| LEA | 2500 | 2500 | 0 | 0 |
| LINK | 2500 | 2500 | 0 | 0 |
| LSL.b | 2500 | 2500 | 0 | 0 |
| LSL.l | 2500 | 2500 | 0 | 0 |
| LSL.w | 2500 | 2500 | 0 | 0 |
| LSR.b | 2500 | 2500 | 0 | 0 |
| LSR.l | 2500 | 2500 | 0 | 0 |
| LSR.w | 2500 | 2500 | 0 | 0 |
| MOVE.b | 2500 | 2500 | 0 | 0 |
| MOVE.l | 2500 | 2500 | 0 | 0 |
| MOVE.q | 2500 | 2500 | 0 | 0 |
| MOVE.w | 2500 | 2500 | 0 | 0 |
| MOVEA.l | 2500 | 2500 | 0 | 0 |
| MOVEA.w | 2500 | 2500 | 0 | 0 |
| MOVEM.l | 2500 | 2500 | 0 | 0 |
| MOVEM.w | 2500 | 2500 | 0 | 0 |
| MOVEP.l | 2500 | 2500 | 0 | 0 |
| MOVEP.w | 2500 | 2500 | 0 | 0 |
| MOVEfromSR | 2500 | 2500 | 0 | 0 |
| MOVEfromUSP | 2500 | 2500 | 0 | 0 |
| MOVEtoCCR | 2500 | 2500 | 0 | 0 |
| MOVEtoSR | 2500 | 2500 | 0 | 0 |
| MOVEtoUSP | 2500 | 2500 | 0 | 0 |
| MULS | 2500 | 2500 | 0 | 0 |
| MULU | 2500 | 2500 | 0 | 0 |
| NBCD | 2500 | 2500 | 0 | 0 |
| NEG.b | 2500 | 2500 | 0 | 0 |
| NEG.l | 2500 | 2500 | 0 | 0 |
| NEG.w | 2500 | 2500 | 0 | 0 |
| NEGX.b | 2500 | 2500 | 0 | 0 |
| NEGX.l | 2500 | 2500 | 0 | 0 |
| NEGX.w | 2500 | 2500 | 0 | 0 |
| NOP | 2500 | 2500 | 0 | 0 |
| NOT.b | 2500 | 2500 | 0 | 0 |
| NOT.l | 2500 | 2500 | 0 | 0 |
| NOT.w | 2500 | 2500 | 0 | 0 |
| OR.b | 2500 | 2500 | 0 | 0 |
| OR.l | 2500 | 2500 | 0 | 0 |
| OR.w | 2500 | 2500 | 0 | 0 |
| ORItoCCR | 2500 | 2500 | 0 | 0 |
| ORItoSR | 2500 | 2500 | 0 | 0 |
| PEA | 2500 | 2500 | 0 | 0 |
| RESET | 2500 | 2500 | 0 | 0 |
| ROL.b | 2500 | 2500 | 0 | 0 |
| ROL.l | 2500 | 2500 | 0 | 0 |
| ROL.w | 2500 | 2500 | 0 | 0 |
| ROR.b | 2500 | 2500 | 0 | 0 |
| ROR.l | 2500 | 2500 | 0 | 0 |
| ROR.w | 2500 | 2500 | 0 | 0 |
| ROXL.b | 2500 | 2500 | 0 | 0 |
| ROXL.l | 2500 | 2500 | 0 | 0 |
| ROXL.w | 2500 | 2500 | 0 | 0 |
| ROXR.b | 2500 | 2500 | 0 | 0 |
| ROXR.l | 2500 | 2500 | 0 | 0 |
| ROXR.w | 2500 | 2500 | 0 | 0 |
| RTE | 2500 | 2500 | 0 | 0 |
| RTR | 2500 | 2500 | 0 | 0 |
| RTS | 2500 | 2500 | 0 | 0 |
| SBCD | 2500 | 2500 | 0 | 0 |
| STOP | 2500 | 2500 | 0 | 0 |
| SUB.b | 2500 | 2500 | 0 | 0 |
| SUB.l | 2500 | 2500 | 0 | 0 |
| SUB.w | 2500 | 2500 | 0 | 0 |
| SUBA.l | 2500 | 2500 | 0 | 0 |
| SUBA.w | 2500 | 2500 | 0 | 0 |
| SUBX.b | 2500 | 2500 | 0 | 0 |
| SUBX.l | 2500 | 2500 | 0 | 0 |
| SUBX.w | 2500 | 2500 | 0 | 0 |
| SWAP | 2500 | 2500 | 0 | 0 |
| Scc | 2500 | 2500 | 0 | 0 |
| TAS | 2500 | 2500 | 0 | 0 |
| TRAP | 2500 | 2500 | 0 | 0 |
| TRAPV | 2500 | 2500 | 0 | 0 |
| TST.b | 2500 | 2500 | 0 | 0 |
| TST.l | 2500 | 2500 | 0 | 0 |
| TST.w | 2500 | 2500 | 0 | 0 |
| UNLINK | 2500 | 2500 | 0 | 0 |
