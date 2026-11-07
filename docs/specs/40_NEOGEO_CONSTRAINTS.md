# 40 - CONTRAINTES NEOGEO (futur projet puce-neogeo = puce-z80 + puce-m68k)

Statut: contraintes de compatibilite imposees a `puce-m68k` pour qu'il serve de CPU principal d'une NeoGeo (AES/MVS). Source: wiki NeoGeo Dev (https://wiki.neogeodev.org/). Les points marques (a confirmer) viennent de connaissances generales: les pages du wiki correspondantes n'ont pas pu etre lues.

## 1. Interruptions du 68000 (source: wiki, page 68k interrupts)

- Cartouche (AES/MVS): trois niveaux **autovectorises**: niveau 1 = V-blank, niveau 2 = timer (compteur 32 bits cadence par l'horloge pixel de 6 MHz), niveau 3 = pendant apres le reset (demarrage a froid).
- L'acquittement des interruptions video se fait par une ecriture dans `REG_IRQACK` (bit 2 V-blank, bit 1 timer, bit 0 reset): c'est du **materiel de la machine**, pas du CPU.
- Le masque est `SR.IMASK` (bits 8-10 de SR). Le CPU doit donc implementer: broches IPL0-2, comparaison avec IMASK a la frontiere d'instruction, cycle d'acquittement (FC = 7), autovecteur via VPA (vecteurs 25..27 pour les niveaux 1..3).
- Consequence: **E11-02 (traitement d'exception), E11-05 (IPL + autovecteur) et E11-06 (RESET)** ne sont plus differees (D-032). Le CD (NeoGeo CD) utilise six interruptions vectorisees: hors perimetre, mais l'interface IPL ne doit pas l'interdire (un hote doit pouvoir fournir un numero de vecteur).

## 2. Horloges et ordonnancement (a confirmer)

- Horloge maitre d'environ 24 MHz; 68000 a 12 MHz (maitre / 2), Z80 a 4 MHz (maitre / 6), horloge pixel 6 MHz (maitre / 4).
- `puce-m68k` et `puce-z80` avancent d'un `tick` = une horloge de leur propre cadence (D-002): le futur `puce-neogeo` possede l'ordonnanceur sur l'horloge maitre; **aucun des deux coeurs ne contient d'horloge ni de diviseur**.

## 3. Wait states du bus 68000

- Les attentes de la NeoGeo (acces ROM, VRAM, synchronisation avec le video) sont decidees par la machine. `Bus::wait_clocks(addr, write)` est evalue au DEBUT du cycle: suffisant pour une attente fixe; si l'attente depend d'une phase plus tardive, l'hote pilote DTACK lui-meme via `Cpu::tick(pins)`. Les deux interfaces doivent rester disponibles (ne pas supprimer `tick(pins)` au profit de `Bus`).

## 4. Liaison 68000 <-> Z80 (a confirmer)

- Le 68000 envoie des commandes au Z80 par un verrou (latch) qui declenche la NMI du Z80; le YM2610 declenche l'INT du Z80. Le crate Z80 gere deja NMI (front), INT, WAIT et BUSRQ: aucune modification de `puce-m68k` n'est necessaire; la logique du verrou est dans la machine.

## 5. Regles d'interface communes aux deux coeurs

- `Pins` = `u64`, logique positive, `tick(pins) -> pins`; coeur `no_std`, sans alloc, sans global; un `Cpu` possede tout son etat (plusieurs instances possibles).
- Les types portent le meme nom dans les deux crates (`Cpu`, `Pins`, `Bus`): toujours les referencer par leur crate (`puce_z80::Cpu`, `puce_m68k::Cpu`).
- Sauvegarde d'etat: le Z80 a `Cpu::save_state` / `from_state`; `CpuState` du 68000 (E03-03) doit pouvoir devenir un etat complet serialisable (moteur compris) sans casser l'API: reserve un champ prive d'etat du moteur plutot que d'exposer des registres internes.
- Determinisme: memes entrees, memes sorties, sur toutes les plateformes (entiers uniquement).

## 6. Hors perimetre CPU

Processeur graphique (LSPC), YM2610, cartouche (bank-switching, protections), BIOS, entrees/sorties: autre projet.
