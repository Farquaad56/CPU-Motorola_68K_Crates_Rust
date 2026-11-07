# 31 - PROTOCOLE FRONTEND <-> SERVEUR (WebSocket JSON)

Serveur `m68k-web`: `http://127.0.0.1:8080/` sert la page (`docs/frontend/index.html`, Tailwind, FR/EN, embarquee avec `include_str!`),
`ws://127.0.0.1:8080/ws` = WebSocket. Messages = textes JSON. Port: `m68k-web --port N`. Ecoute 127.0.0.1 uniquement. La page ne calcule rien (sauf mise en forme).

## 1. Client -> serveur

| `cmd` | Champs | Effet |
|---|---|---|
| `hello` | - | repond `{"type":"hello","version":"<crate>"}` puis `roms`, `state` |
| `list_roms` | - | repond `roms` (fichiers `tests/roms/**`: `.bin` brut) |
| `load` | `name` | charge le binaire en 0x000000 de la RAM plate; SSP et PC pris dans les vecteurs 0 et 1 (chargement direct des registres, pas d'exception RESET), reset de l'etat |
| `upload` | `name`, `data_b64` | idem avec des octets envoyes par la page |
| `run` / `pause` | - | demarre / arrete la simulation (thread serveur) |
| `step` | `n` (defaut 1) | execute n instructions puis `state` |
| `step_clock` | `n` (defaut 1) | execute n horloges puis `state` (debogage cycle par cycle) |
| `reset` | - | recharge le binaire courant |
| `speed` | `idx` 0..7 | instructions par seconde: [1,4,15,60,300,3000,60000,0=turbo] |
| `bp_add` / `bp_del` | `addr` | points d'arret |
| `view` | `mem_base`, `dis_rows`, `trace_rows`, `stack_rows`, `follow` | fenetre d'observation (defauts 256, 40, 16, 14); `follow` in pc, sp, an0..an6, man |
| `sst_list` | - | repond `sst_files` (stems de `tests/singlestep`) |
| `sst_load` | `stem`, `index` | charge le test n de ce fichier (etat initial dans le CPU) et renvoie `sst_test` |
| `console_clear` | - | vide la console |

## 2. Serveur -> client

`{"type":"roms","roms":[{"name","size"}]}` ; `{"type":"sst_files","files":[{"stem","tests"}]}`

`{"type":"state", ...}` au plus 30 fois/s pendant `run`, et immediatement apres chaque commande:

```
status: "idle"|"running"|"halted"|"done"|"bp"      instrs, clocks (entiers 64 bits), ips, mhz (nombres)
regs: {d:[8 entiers], a:[8 entiers] (a[7] = pile active), usp, ssp, sr, pc, ir, irc, x, n, z, v, c, s, t, imask}
bus:  {kind: "n"|"r"|"w"|"t", addr, data, fc, uds, lds, clocks, pins: "0x..." (u64 de Pins)}     dernier cycle de bus
buslog: [{kind, addr, data, fc, uds, lds, clocks}]   32 derniers cycles, plus recent en premier   (feature inspect)
dis:  [{addr, words:[...], text, len}]   dis_rows lignes a partir de PC (desassembleur du crate)
trace:[{pc, text}]    stack:[{addr, word}]
mem:  {base, hex: "<512 caracteres hex>"}      wlog: [adresses ecrites recemment]
console_delta: "texte ajoute depuis le dernier state"
bps: [adresses]       rom: {name, size}
```

## 3. Debogage cycle par cycle (SingleStepTests)

`{"type":"sst_test","name","expected":[{kind,cycles,fc,addr,data,uds,lds}],"actual":[...],"diff":[indices differents]}` apres `sst_load` puis chaque `step`/`step_clock`: la page affiche les deux colonnes de transactions et surligne l'ecart.

Regles: aucune valeur n'est calculee par la page. Les mnemoniques viennent du crate (`disasm`). La page tolere des champs absents.
