# 00 - METHODE (heritee de Puce16/Puce8)

1. **Docs d'abord**: tout ce qu'une tache doit savoir est dans `docs/specs/` (et recopie dans la tache). Aucune connaissance implicite.
2. **Taches minuscules** (50-150 lignes), une seule chaine stricte (D-020), tests nommes `<id>_<suffixe>`, acceptance = commandes shell.
3. **Oracle hors-ligne**: Hermes n'a pas besoin du reseau pour valider la semantique: les 127 fichiers `tests/singlestep/*.json.bin` (317500 tests SingleStepTests/m68000, etat initial/final + cycles de bus) sont telecharges une fois (`python docs/tools/fetch_refs.py`), puis tout est local. Les specs par instruction (`docs/specs/instr/`) sont extraites de Yacht.txt et du manuel (E04).
4. **Jamais deviner**: valeur inconnue -> commentaire `UNKNOWN - to confirm` + question Q&R (`docs/02_QA_PROTOCOL.md`), la tache est bloquee, Claude/l'utilisateur repond.
5. **GATE**: points de controle humains (fin d'epopee): l'utilisateur lance les commandes indiquees puis `task.py approve`.
6. **Hygiene**: ASCII, anglais dans le code, pas de `unsafe`, pas de dependance non listee, pas de `f32/f64` dans `puce-m68k`.
7. **Git**: Hermes ne committe jamais. L'utilisateur committe (sans `Co-Authored-By` ni `Claude-Session` si c'est la regle du projet).
8. **Regle d'or de la reutilisation**: le crate n'a pas d'horloge, pas d'etat global, pas d'allocation; tout I/O passe par `Pins` ou le trait `Bus`.
