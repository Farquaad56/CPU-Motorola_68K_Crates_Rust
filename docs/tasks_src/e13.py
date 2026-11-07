from _dsl import T

W = "puce-m68k-web"
MS = "puce-m68k-machines"
PROTO = ["31_FRONTEND_PROTOCOL.md#1. Client -> serveur", "31_FRONTEND_PROTOCOL.md#2. Serveur -> client"]

T("E13-01", "Session: commandes et boucle de simulation", kind="code", size=150, crate=MS,
  context="Thread de simulation partage par le serveur web: pilote un `Cpu` reel sur la RAM plate, commandes run/pause/step/step_clock/reset/speed/points d'arret.",
  embed=["31_FRONTEND_PROTOCOL.md#1. Client -> serveur"],
  spec="Dans `crates/puce-m68k-machines/src/session.rs`: `pub enum Command {...}` (un variant par `cmd` du protocole), `pub struct Session` (thread + canaux `std::sync::mpsc`), `Session::spawn(roms_dir) -> Session`, `send(Command)`, `latest_state() -> Snapshot`. Boucle: en `run`, execute des instructions par lots selon `speed` et s'arrete sur point d'arret.",
  write=["crates/puce-m68k-machines/src/session.rs"],
  tests=[("step", "step 1 sur un programme `nop` avance le PC de 2"), ("step_clock", "step_clock 4 sur un NOP: 4 horloges"), ("breakpoint", "run s'arrete sur le point d'arret (status bp)"), ("pause", "pause arrete la simulation"), ("reset", "reset recharge SSP/PC des vecteurs"), ("speed_index", "speed hors 0..7 est refuse")])
T("E13-02", "Session: instantane d'etat conforme au protocole", kind="code", size=150, crate=MS,
  context="Construit le `Snapshot` (struct serde-free: JSON ecrit a la main) avec les champs de la section 2 du protocole.",
  embed=["31_FRONTEND_PROTOCOL.md#2. Serveur -> client"],
  spec="`Snapshot::to_json(&self) -> String` sans dependance (echappement des chaines, entiers, tableaux). Champs: status, instrs, clocks, regs, bus, buslog, dis (via `disasm`), trace, stack, mem, wlog, console_delta, bps, rom. Dependance `serde_json` autorisee seulement si une question Q&R est validee: sinon ecris l'encodeur a la main.",
  write=["crates/puce-m68k-machines/src/session.rs", "crates/puce-m68k-machines/src/snapshot.rs", "crates/puce-m68k-machines/src/lib.rs"],
  tests=[("json_valid_regs", "le JSON contient d, a, sr, pc"), ("escape", "une chaine avec guillemet est echappee"), ("dis_rows", "dis_rows=5 donne 5 lignes"), ("mem_hex", "mem.hex a 512 caracteres"), ("buslog_order", "buslog plus recent en premier"), ("missing_rom", "sans ROM: rom = null, pas d'erreur")])
T("E13-03", "Serveur HTTP: page et sante", kind="code", size=100, crate=W,
  context="Serveur HTTP minimal sur 127.0.0.1 servant la page embarquee. Dependances autorisees: `tiny_http`.",
  embed=["31_FRONTEND_PROTOCOL.md#1. Client -> serveur"],
  spec="`GET /` renvoie `docs/frontend/index.html` embarque (`include_str!`), `GET /health` renvoie `ok`, autre chemin -> 404. Ecoute 127.0.0.1 uniquement.",
  write=["crates/puce-m68k-web/src/http.rs", "crates/puce-m68k-web/src/main.rs", "crates/puce-m68k-web/Cargo.toml", "docs/frontend/index.html", "Cargo.lock"],
  tests=[("index_ok", "GET / = 200 et contient `<title>`"), ("health", "GET /health = ok"), ("not_found", "GET /x = 404"), ("localhost_only", "l'adresse d'ecoute est 127.0.0.1")])
T("E13-04", "Serveur WebSocket (/ws)", kind="code", size=150, crate=W,
  context="Mise a niveau WebSocket et boucle message/etat. Dependances autorisees: `tungstenite`, `base64`.",
  embed=PROTO,
  spec="`ws.rs`: accepte `/ws`, lit les messages JSON clients (analyse a la main des champs `cmd`, `n`, `addr`...), pousse `state` apres chaque commande et au plus 30 fois/s en run.",
  write=["crates/puce-m68k-web/src/ws.rs", "crates/puce-m68k-web/src/main.rs", "crates/puce-m68k-web/Cargo.toml", "Cargo.lock"],
  tests=[("hello", "`hello` renvoie hello puis roms puis state"), ("unknown_cmd", "cmd inconnue -> message d'erreur, connexion gardee"), ("rate_limit", "pas plus de 30 etats/s en run"), ("bad_json", "JSON invalide -> erreur, pas de panic")])
T("E13-05", "Commandes: roms, load, upload, sst", kind="code", size=140, crate=W,
  context="Chargement des binaires et navigation dans les tests SingleStepTests.",
  embed=["31_FRONTEND_PROTOCOL.md#3. Debogage"],
  spec="`list_roms`, `load`, `upload` (base64), `sst_list`, `sst_load` (charge le test n, renvoie `sst_test` avec `expected`/`actual`/`diff`).",
  write=["crates/puce-m68k-web/src/ws.rs", "crates/puce-m68k-machines/src/session.rs"],
  tests=[("list_roms", "liste les .bin du dossier"), ("load_unknown", "nom inconnu -> erreur"), ("upload_b64", "octets decodes et charges"), ("sst_list", "retourne les stems avec le nombre de tests"), ("sst_load_diff", "apres sst_load d'un test NOP et un step, `diff` liste les transactions fausses")])
T("E13-06", "Page web Tailwind: panneaux et protocole", kind="ui", size=150, crate=W,
  context="Page unique `docs/frontend/index.html` (Tailwind via CDN comme la page Z80, interface FR/EN, aucun calcul CPU en JS). Point de depart: `docs/frontend/z80_reference.html` (a adapter: registres 68000, bus a UDS/LDS/FC, desassembleur).",
  embed=PROTO,
  spec="Panneaux: registres D0-D7/A0-A7/USP/SSP/SR(drapeaux)/PC/IR/IRC, desassembleur autour du PC, memoire hex (follow), pile, trace, journal de bus (32 cycles), console, controles (run, pause, step, step horloge, reset, vitesse, points d'arret), selecteur de binaire + glisser-deposer, panneau SingleStepTests (liste de fichiers, numero de test, colonnes attendu/obtenu avec ecart surligne). Titre `Motorola 68000 Emulator`.",
  write=["docs/frontend/index.html", "crates/puce-m68k-web/tests/frontend_contract.rs"],
  tests=[("title", "le HTML contient `Motorola 68000 Emulator`"), ("tailwind", "le HTML charge Tailwind"), ("ws_fields", "chaque champ du protocole section 2 utilise par la page est cite dans le HTML"), ("fr_en", "les deux langues ont les memes cles de traduction"), ("no_cpu_logic", "le HTML ne contient aucun mnemonique de calcul (pas d'ALU en JS: grep de `0x...` interdit hors affichage)")],
  accept=["cargo test -p puce-m68k-web frontend_contract"])
T("E13-07", "Test bout en bout du serveur (client Rust)", kind="code", size=140, crate=W,
  context="Test d'integration: lance le serveur sur un port libre, se connecte en WebSocket, execute un programme.",
  spec="`crates/puce-m68k-web/tests/e2e.rs`: demarre le binaire/serveur, `hello`, `load` d'un petit programme, `step` x3, verifie `regs.pc` et `clocks`; `sst_load` NOP.",
  write=["crates/puce-m68k-web/tests/e2e.rs"],
  tests=[("hello_state", "hello donne un state valide"), ("load_step", "load + step x3 avance le PC"), ("clocks", "les horloges correspondent a la somme des plans"), ("sst_roundtrip", "sst_load NOP renvoie 2500 tests dans sst_files")])
T("E13-08", "Binaire m68k-web: demarrage et arret propre", kind="code", size=70, crate=W,
  context="Assemblage final du serveur.",
  spec="`main`: affiche l'URL, demarre HTTP+WS+Session, arret propre sur Ctrl-C (fermeture des sockets, thread de simulation arrete).",
  write=["crates/puce-m68k-web/src/main.rs"],
  tests=[("bind_error", "port deja pris -> message clair, code 1"), ("shutdown_flag", "le drapeau d'arret termine la boucle"), ("banner", "la banniere contient l'URL")])
T("E13-09", "GATE E13 - serveur et page valides", gate=True, kind="gate", size=0,
  context="Validation humaine du frontend: ouvrir la page et pousser un programme.",
  user_cmds=["cargo run --release -p puce-m68k-web", "ouvrir http://127.0.0.1:8080/ ; charger un binaire; Step; verifier registres, desassembleur et journal de bus", "cargo test -p puce-m68k-web"], accept=[])
