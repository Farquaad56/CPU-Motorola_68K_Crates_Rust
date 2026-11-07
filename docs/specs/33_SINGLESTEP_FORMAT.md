# 33 - FORMAT SingleStepTests/m68000 (`.json.bin`)

Source: README du depot + lecture de `docs/ref/singlestep/decode.py` (le script n'est PAS execute par le projet: le lecteur est reecrit en Rust, E03-01). Tous les entiers sont little-endian.

## 1. Fichier

```
u32 magic = 0x1A3F5D71
u32 num_tests            (2500 dans tous les fichiers v1)
test * num_tests
```

## 2. Un test

```
u32 numbytes, u32 magic = 0xABC12367
nom:      u32 numbytes, u32 magic = 0x89ABCDEF, u32 strlen, strlen octets UTF-8
etat initial   (voir 3)
etat final     (voir 3)
transactions   (voir 4)
```
Le lecteur parcourt sequentiellement et ignore `numbytes` (UNKNOWN - to confirm: si `numbytes` inclut ou non l'en-tete de 8 octets; sans importance tant qu'on lit champ par champ).

## 3. Etat

```
u32 numbytes, u32 magic = 0x01234567
19 x u32 dans l'ordre: d0..d7, a0..a6, usp, ssp, sr, pc
u32 prefetch0, u32 prefetch1        (file de prefetch: IR puis IRC)
u32 num_rams
num_rams x { u32 addr (pair, < 0x1000000), u16 data }    mot de 16 bits a l'adresse addr (octet haut a addr, bas a addr+1)
```
- `pc` = valeur de `m_au` de MAME = **adresse du prochain mot a prefetcher** = adresse de l'instruction + 4 (l'instruction et son premier mot d'extension sont deja dans la file `prefetch0`, `prefetch1`).
- Seule la memoire touchee par le test est listee; le reste est ignore (le lecteur doit fournir une RAM dont le contenu hors liste est sans effet sur le resultat).
- `a7` n'existe pas dans l'ordre: A7 est `usp` ou `ssp` selon le bit S de `sr`.

## 4. Transactions

```
u32 numbytes, u32 magic = 0x456789AB
u32 num_cycles            (= duree totale en horloges, champ `length` du test)
u32 num_transactions
num_transactions x:
    u8  kind, u32 cycles
    si kind != 0: u32 fc, u32 addr_bus, u32 data_bus, u32 uds, u32 lds
```

| kind | Nom | Sens |
|---|---|---|
| 0 | `n` | cycle interne de `cycles` horloges (pas d'autres champs) |
| 1 | `w` | ecriture |
| 2 | `r` | lecture |
| 3 | `t` | cycle TAS (RMW) |
| 4 | `re` | lecture avec address error (AS non asserte, resultat non committe) |
| 5 | `we` | ecriture avec address error |

- `data_bus` suit le bus reel: octet par UDS seul -> 0xAB00 (voir 11_PINS_AND_TICK.md).
- Taille `.w`/`.b` deduite de `uds + lds` (2 = mot).
- UNKNOWN - to confirm (a lever dans E03-01 en lisant les premiers tests): `addr_bus` est-il une adresse d'octet ou de mot (A1..A23)? `cycles` d'une transaction de bus est-il toujours >= 4?

## 5. Comparaison attendue (E03-03)

Apres execution de `num_cycles` horloges depuis l'etat initial: tous les registres de l'etat final, la file de prefetch, la memoire listee dans l'etat final, et la liste des transactions (kind, cycles, fc, addr, data, uds, lds) doivent etre identiques.
