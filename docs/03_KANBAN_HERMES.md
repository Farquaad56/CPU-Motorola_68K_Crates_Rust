# 03 - KANBAN HERMES (import et exploitation)

Profil Hermes: `M68K_Crate` (assignee). Board: `m68k`. Tenant: `m68k`. Workspace: `H:/__Emulator__/CPU Motorola_68K_Crates_Rust` (chemin avec espaces: toujours entre guillemets).

## Creation du board (une fois)
```
hermes kanban boards create m68k --name "M68K Crate"
hermes kanban boards switch m68k
```
(Adapter a la CLI installee: `hermes kanban --help`.)

## Import
```
python docs/tools/build_tasks.py
python docs/tools/kanban_import.py --dry-run --limit 5
python docs/tools/kanban_import.py --limit 10      # puis relancer: reprise automatique, pas de doublon (cle m68k-<ID>)
```
- Les taches GATE n'ont pas d'assignee (validation humaine). Les taches `deferred` ne sont pas importees (`--include-deferred`).
- Chaque carte pointe vers `docs/tasks/<ID>.md` si le corps depasse la limite (`--max-body`).

## Exploitation
- Cycle: Hermes execute -> `DONE <ID>` -> l'utilisateur lance `python docs/tools/task.py check <ID>` puis `done <ID>` (GATE: `approve <ID>`) -> commit.
- Question bloquante: carte bloquee par Hermes (`kanban_block`), reponse via `python docs/tools/qa.py answer ...` puis relance.
- `python docs/tools/task.py next|status|list|show|scope <ID>` pour piloter.
- Aucun parallelisme: la chaine est stricte (D-020).
