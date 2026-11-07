# 05 - TUTORIEL (pas a pas)

0. Prerequis: Rust stable (rustup), Python 3.8+, Hermes installe avec le profil `M68K_Crate`, acces reseau une fois pour `python docs/tools/fetch_refs.py` (GATE E04-01).
1. Ouvrir un terminal dans `H:\__Emulator__\CPU Motorola_68K_Crates_Rust` (guillemets autour du chemin).
2. `python docs/tools/build_tasks.py --check` doit afficher `OK`.
3. Creer le board et importer (voir `03_KANBAN_HERMES.md`), 5 taches d'abord.
4. E01-01 a E01-06 sont deja faites (par Claude): lancer `python docs/tools/task.py check E01-01` ... `E01-06` puis `done`, executer les commandes de E01-07 et `approve E01-07`, puis `git add -A && git commit`. Hermes commence ensuite a E02-01 (voir `python docs/tools/task.py next`).
5. Repeter. A chaque GATE (E01-07, E02-07, E03-08, E04-04, E05-06, E06-99, ...) executer les commandes indiquees dans la carte.
6. En cas de blocage: lire `docs/qa/inbox/`, repondre avec `python docs/tools/qa.py` (voir `02_QA_PROTOCOL.md`).
7. Les prompts manuels (sans Kanban) sont dans `04_PROMPT_TEMPLATES.md`.
8. Regeneration apres modification des specs/DSL: `python docs/tools/build_tasks.py` (n'ecrase jamais `PROGRESS.json`).

## Profil Hermes M68K_Crate (a faire AVANT l'import)
Toutes les taches sont assignees au profil `M68K_Crate`. Cree-le et branche-le sur LM Studio:
```
hermes profile create M68K_Crate
hermes profile describe M68K_Crate --text "Code Rust du crate M68K cycle-accurate (puce-m68k) a partir des docs/tasks/*.md, une tache a la fois"
hermes -p M68K_Crate config
```
Dans la config: fournisseur = endpoint compatible OpenAI (custom), base URL `http://localhost:1234/v1` (serveur local de LM Studio, bouton "Start Server"),
cle API quelconque (ex. `lm-studio`), modele = `qwen3.8-27b-turbo-fable-cold-fusion-735-882-heretic-uncensored-neo-coder-max-mtp`
(identifiant exact tel qu'affiche par LM Studio, onglet Developer). Active les outils fichiers + terminal. Contexte >= 32k.
Syntaxe exacte des sous-commandes `profile`/`config`: UNKNOWN - to confirm (`hermes profile --help`).
Verification: `hermes profile list` doit montrer `M68K_Crate`, puis `python docs/tools/kanban_import.py --dry-run --limit 5`.
