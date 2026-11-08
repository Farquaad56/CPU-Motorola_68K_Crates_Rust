# Rapport des suites SingleStepTests (m68000)

Genere par `m68k ssc tests/singlestep --json tests/baseline.json` (carte E15-02).

## Resume

- Fichiers: 127 (2500 cas chacun)
- Cas: 317500, reussis: 261894, echoues: 0, ignores (erreur d adresse): 55606
- Cas executes: 261894, dont 261894 reussis (100.00 %)

## Echecs

Aucun: tous les cas executes passent. `TAS` est compte comme reussi depuis D-051 (le banc compare la lecture du cycle TAS comme une lecture simple, comme l oracle). La liste `docs/specs/data/sst_known_exceptions.txt` est vide.

## Cas ignores

Les cas qui declenchent une erreur d adresse sont ignores par le banc (generation du cadre d erreur d adresse: E11-03, differee). Ils sont comptes dans `skip_addr`.

## Reference et regression

`tests/baseline.json` fixe le resultat attendu par fichier. `m68k ssc tests/singlestep --strict --compare tests/baseline.json` retourne le code 1 si un fichier perd des cas reussis ou disparait.

## Detail par fichier

| Fichier | Total | Reussis | Echecs | Ignores |
|---|---:|---:|---:|---:|
| ABCD | 2500 | 2500 | 0 | 0 |
| ADD.b | 2500 | 2500 | 0 | 0 |
| ADD.l | 2500 | 1564 | 0 | 936 |
| ADD.w | 2500 | 1553 | 0 | 947 |
| ADDA.l | 2500 | 1675 | 0 | 825 |
| ADDA.w | 2500 | 1683 | 0 | 817 |
| ADDX.b | 2500 | 2500 | 0 | 0 |
| ADDX.l | 2500 | 1671 | 0 | 829 |
| ADDX.w | 2500 | 1683 | 0 | 817 |
| AND.b | 2500 | 2500 | 0 | 0 |
| AND.l | 2500 | 1500 | 0 | 1000 |
| AND.w | 2500 | 1431 | 0 | 1069 |
| ANDItoCCR | 2500 | 2500 | 0 | 0 |
| ANDItoSR | 2500 | 2500 | 0 | 0 |
| ASL.b | 2500 | 2500 | 0 | 0 |
| ASL.l | 2500 | 2500 | 0 | 0 |
| ASL.w | 2500 | 2221 | 0 | 279 |
| ASR.b | 2500 | 2500 | 0 | 0 |
| ASR.l | 2500 | 2500 | 0 | 0 |
| ASR.w | 2500 | 2217 | 0 | 283 |
| BCHG | 2500 | 2500 | 0 | 0 |
| BCLR | 2500 | 2500 | 0 | 0 |
| BSET | 2500 | 2500 | 0 | 0 |
| BSR | 2500 | 1229 | 0 | 1271 |
| BTST | 2500 | 2500 | 0 | 0 |
| Bcc | 2500 | 1830 | 0 | 670 |
| CHK | 2500 | 1523 | 0 | 977 |
| CLR.b | 2500 | 2500 | 0 | 0 |
| CLR.l | 2500 | 1529 | 0 | 971 |
| CLR.w | 2500 | 1481 | 0 | 1019 |
| CMP.b | 2500 | 2500 | 0 | 0 |
| CMP.l | 2500 | 1547 | 0 | 953 |
| CMP.w | 2500 | 1561 | 0 | 939 |
| CMPA.l | 2500 | 1670 | 0 | 830 |
| CMPA.w | 2500 | 1664 | 0 | 836 |
| DBcc | 2500 | 1868 | 0 | 632 |
| DIVS | 2500 | 1518 | 0 | 982 |
| DIVU | 2500 | 1546 | 0 | 954 |
| EOR.b | 2500 | 2500 | 0 | 0 |
| EOR.l | 2500 | 1534 | 0 | 966 |
| EOR.w | 2500 | 1515 | 0 | 985 |
| EORItoCCR | 2500 | 2500 | 0 | 0 |
| EORItoSR | 2500 | 2500 | 0 | 0 |
| EXG | 2500 | 2500 | 0 | 0 |
| EXT.l | 2500 | 2500 | 0 | 0 |
| EXT.w | 2500 | 2500 | 0 | 0 |
| ILLEGAL_LINEA | 2500 | 2500 | 0 | 0 |
| ILLEGAL_LINEF | 2500 | 2500 | 0 | 0 |
| JMP | 2500 | 1272 | 0 | 1228 |
| JSR | 2500 | 1341 | 0 | 1159 |
| LEA | 2500 | 2500 | 0 | 0 |
| LINK | 2500 | 2500 | 0 | 0 |
| LSL.b | 2500 | 2500 | 0 | 0 |
| LSL.l | 2500 | 2500 | 0 | 0 |
| LSL.w | 2500 | 2238 | 0 | 262 |
| LSR.b | 2500 | 2500 | 0 | 0 |
| LSR.l | 2500 | 2500 | 0 | 0 |
| LSR.w | 2500 | 2212 | 0 | 288 |
| MOVE.b | 2500 | 2500 | 0 | 0 |
| MOVE.l | 2500 | 1013 | 0 | 1487 |
| MOVE.q | 2500 | 2500 | 0 | 0 |
| MOVE.w | 2500 | 1013 | 0 | 1487 |
| MOVEA.l | 2500 | 1655 | 0 | 845 |
| MOVEA.w | 2500 | 1658 | 0 | 842 |
| MOVEM.l | 2500 | 1304 | 0 | 1196 |
| MOVEM.w | 2500 | 1318 | 0 | 1182 |
| MOVEP.l | 2500 | 2500 | 0 | 0 |
| MOVEP.w | 2500 | 2500 | 0 | 0 |
| MOVEfromSR | 2500 | 1522 | 0 | 978 |
| MOVEfromUSP | 2500 | 2500 | 0 | 0 |
| MOVEtoCCR | 2500 | 1504 | 0 | 996 |
| MOVEtoSR | 2500 | 2056 | 0 | 444 |
| MOVEtoUSP | 2500 | 2500 | 0 | 0 |
| MULS | 2500 | 1548 | 0 | 952 |
| MULU | 2500 | 1501 | 0 | 999 |
| NBCD | 2500 | 2500 | 0 | 0 |
| NEG.b | 2500 | 2500 | 0 | 0 |
| NEG.l | 2500 | 1549 | 0 | 951 |
| NEG.w | 2500 | 1543 | 0 | 957 |
| NEGX.b | 2500 | 2500 | 0 | 0 |
| NEGX.l | 2500 | 1543 | 0 | 957 |
| NEGX.w | 2500 | 1537 | 0 | 963 |
| NOP | 2500 | 2500 | 0 | 0 |
| NOT.b | 2500 | 2500 | 0 | 0 |
| NOT.l | 2500 | 1471 | 0 | 1029 |
| NOT.w | 2500 | 1509 | 0 | 991 |
| OR.b | 2500 | 2500 | 0 | 0 |
| OR.l | 2500 | 1423 | 0 | 1077 |
| OR.w | 2500 | 1438 | 0 | 1062 |
| ORItoCCR | 2500 | 2500 | 0 | 0 |
| ORItoSR | 2500 | 2500 | 0 | 0 |
| PEA | 2500 | 2500 | 0 | 0 |
| RESET | 2500 | 2500 | 0 | 0 |
| ROL.b | 2500 | 2500 | 0 | 0 |
| ROL.l | 2500 | 2500 | 0 | 0 |
| ROL.w | 2500 | 2209 | 0 | 291 |
| ROR.b | 2500 | 2500 | 0 | 0 |
| ROR.l | 2500 | 2500 | 0 | 0 |
| ROR.w | 2500 | 2203 | 0 | 297 |
| ROXL.b | 2500 | 2500 | 0 | 0 |
| ROXL.l | 2500 | 2500 | 0 | 0 |
| ROXL.w | 2500 | 2228 | 0 | 272 |
| ROXR.b | 2500 | 2500 | 0 | 0 |
| ROXR.l | 2500 | 2500 | 0 | 0 |
| ROXR.w | 2500 | 2224 | 0 | 276 |
| RTE | 2500 | 1886 | 0 | 614 |
| RTR | 2500 | 1285 | 0 | 1215 |
| RTS | 2500 | 1263 | 0 | 1237 |
| SBCD | 2500 | 2500 | 0 | 0 |
| STOP | 2500 | 2500 | 0 | 0 |
| SUB.b | 2500 | 2500 | 0 | 0 |
| SUB.l | 2500 | 1599 | 0 | 901 |
| SUB.w | 2500 | 1579 | 0 | 921 |
| SUBA.l | 2500 | 1676 | 0 | 824 |
| SUBA.w | 2500 | 1626 | 0 | 874 |
| SUBX.b | 2500 | 2500 | 0 | 0 |
| SUBX.l | 2500 | 1673 | 0 | 827 |
| SUBX.w | 2500 | 1646 | 0 | 854 |
| SWAP | 2500 | 2500 | 0 | 0 |
| Scc | 2500 | 2500 | 0 | 0 |
| TAS | 2500 | 2500 | 0 | 0 |
| TRAP | 2500 | 2500 | 0 | 0 |
| TRAPV | 2500 | 2500 | 0 | 0 |
| TST.b | 2500 | 2500 | 0 | 0 |
| TST.l | 2500 | 1524 | 0 | 976 |
| TST.w | 2500 | 1507 | 0 | 993 |
| UNLINK | 2500 | 1385 | 0 | 1115 |
