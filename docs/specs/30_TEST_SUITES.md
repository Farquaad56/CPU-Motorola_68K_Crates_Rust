# 30 - SUITES DE TEST

| Suite | Source | Role | Etat |
|---|---|---|---|
| SingleStepTests/m68000 (v1) | https://github.com/SingleStepTests/m68000 (MIT) | oracle principal: etat initial/final + cycles de bus de chaque instruction. 127 fichiers x 2500 tests = 317500 tests, ~137 Mo, binaire `.json.bin` | telecharge (tests/singlestep) |
| m68k_opcode_sizes | https://github.com/realmonster/smd_emu_tests (sources dans docs/ref/smd_emu_tests) | decodage: rejet des opcodes illegaux, taille des instructions; ROM Mega Drive (affichage VDP) | sources seulement |
| Yacht.txt | http://nemesis.hacking-cult.org/MegaDrive/Documentation/Yacht.txt | specification du timing de bus au niveau microcode (pas un test) | `fetch_refs.py` |

## Regles de lecture des resultats SingleStepTests

- Les tests sont generes depuis le coeur microcode de **MAME**, pas depuis du materiel: un bug MAME se retrouve dans les tests (D-007).
- Tests contenant un cycle `re` ou `we` (address error): ignores jusqu'a l'epopee E11 (comptes a part: `skipped_addr_error`).
- Exceptions connues (liste dans `docs/specs/data/sst_known_exceptions.txt`, une ligne `stem<TAB>raison`): TAS (timing RMW), TRAPV (comportement du bit S). Un fichier liste = les echecs de ce stem ne font pas echouer `--strict`.
- Pas de ROM de timing verifiee sur materiel connue publiquement (Nemesis n'a jamais publie la sienne). Cas douteux: recouper avec FX68K (https://github.com/ijor/fx68k) a la main, puis noter la decision dans DECISIONS.md.
