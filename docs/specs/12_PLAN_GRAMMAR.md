# 12 - GRAMMAIRE DES PLANS DE MICRO-CYCLES (BROUILLON)

Statut: **BROUILLON** a valider par E02-04 contre Yacht.txt (docs/ref/Yacht.txt). Toute decision de ce fichier peut etre amendee par une entree de `docs/qa/DECISIONS.md`.

## 1. Principe

Une instruction decodee = un plan `&'static [Tok]` choisi par (instruction, taille, mode source, mode destination). Le moteur consomme un jeton a la fois; chaque jeton dure un nombre fixe d'horloges (plus les wait states pour les jetons de bus). Les calculs (ALU, drapeaux, registres) sont faits par des **hooks** (`H0`, `H1`...) appeles a des ticks precis d'un jeton, comme dans le crate Z80.

## 2. Jetons

| Jeton | Duree (horloges) | Effet bus |
|---|---|---|
| `N(n)` | n (pair) | cycle interne, aucune activite de bus |
| `Pf` | 4 | prefetch: lit le mot a l'adresse `PC` (FC = programme), IRC -> IR puis mot lu -> IRC, `PC += 2` |
| `Rb(addr)` / `Rw(addr)` | 4 | lecture octet / mot a l'adresse source (FC donnees) |
| `Wb(addr,val)` / `Ww(addr,val)` | 4 | ecriture octet / mot |
| `Rl(addr)` / `Wl(addr,val)` | 8 | mot haut puis mot bas (deux cycles de bus enchaines) |
| `Tas` | 4 (RMW) | cycle indivisible lecture-modification-ecriture de TAS (UNKNOWN - to confirm: 5 cycles?) |
| `H(k)` | 0 | appelle le hook `k` (semantique) sans consommer d'horloge |

`addr` est une source d'adresse symbolique: `Pc`, `Ea`, `EaPlus2`, `An(n)`, `AnPre(n)` (pre-decrement), `AnPost(n)` (post-increment), `Sp`, `SpPre`, `SpPost`, `Vec(n)`. UNKNOWN - to confirm: liste definitive (E02-04).

## 3. Regles

- Les accesses de longueur 4 octets sont deux cycles de bus (haut puis bas), sauf `MOVE.l` en ecriture memoire dont l'ordre (bas puis haut pour les modes pre-decrement) est donne par Yacht.txt.
- L'adresse impaire en acces mot/long declenche une address error (E11): avant E11, ces tests sont ignores (cycles `re`/`we`).
- Un plan se termine par un `Pf` qui amene l'instruction suivante dans IR.
