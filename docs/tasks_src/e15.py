from _dsl import T

T("E15-01", "Performance du coeur (release)", kind="code", size=100, crate="puce-m68k-machines",
  context="Mesure la vitesse du coeur; objectif indicatif: au moins 8 MHz equivalent (une horloge 68000 = 1 tick) en release, sans degrader le determinisme.",
  spec="Test `#[ignore]` `crates/puce-m68k-machines/tests/perf.rs` + sous-commande `m68k bench`: execute 50 millions d'horloges d'un programme de boucle MOVE/ADD/Bcc et affiche MHz equivalent. Optimisations autorisees: aucune sans test SingleStepTests inchange.",
  write=["crates/puce-m68k-machines/tests/perf.rs", "crates/puce-m68k-cli/src/commands.rs", "crates/puce-m68k-cli/src/main.rs"],
  tests=[("bench_runs", "la boucle de bench s'execute 1000 horloges sans panique")],
  accept=["cargo test --release -p puce-m68k-machines e15_01_"])
T("E15-02", "Baseline complete SingleStepTests", kind="code", size=100, crate="puce-m68k-cli",
  context="Passe complete des 127 fichiers (hors address errors tant que E11 n'est pas faite) et fichier de reference.",
  spec="`m68k ssc tests/singlestep --json tests/baseline.json` ecrit le resume par fichier (total, pass, fail, skip_addr); `--compare tests/baseline.json` echoue si un fichier regresse. Documente le resultat dans `docs/SUITES_REPORT.md`.",
  write=["crates/puce-m68k-cli/src/commands.rs", "crates/puce-m68k-cli/src/main.rs", "tests/baseline.json", "docs/SUITES_REPORT.md"],
  tests=[("json_format", "le JSON contient un objet par stem"), ("compare_regress", "un fichier qui passe de 2500 a 2499 fait echouer --compare"), ("compare_same", "baseline identique: succes")],
  accept=["cargo run --release -q -p puce-m68k-cli -- ssc tests/singlestep --strict --compare tests/baseline.json"])
T("E15-03", "Documentation du crate et README final", kind="doc", size=0,
  context="Documentation publique: README racine, README du crate, exemples, guide de reutilisation (API pins/tick et trait Bus).",
  spec="`cargo doc --no-deps -p puce-m68k` sans avertissement; `README.md` a jour (tableau du workspace, specifications techniques, resultats des suites, installation, usage); `docs/specs/32_REUSE_GUIDE.md` (integrer le coeur dans un emulateur de console: Mega Drive, Atari ST, Amiga).",
  write=["README.md", "CHANGELOG.md", "crates/puce-m68k/README.md", "docs/specs/32_REUSE_GUIDE.md"],
  accept=["cargo doc --no-deps -p puce-m68k"])
T("E15-04", "GATE FINAL - crate 68000 complet", gate=True, kind="gate", size=0,
  context="Validation humaine finale: toutes les epopees v1 approuvees.",
  user_cmds=["bash tools/release_check.sh", "python docs/tools/task.py status"], accept=[])
