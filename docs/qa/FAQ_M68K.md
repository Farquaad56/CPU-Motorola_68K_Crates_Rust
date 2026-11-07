# FAQ M68K (a consulter avant toute question bloquante)

- **Ou sont les specs par instruction?** `docs/specs/instr/<MNEMONIQUE>.md`, generees en E04 depuis `docs/ref/Yacht.txt` et le manuel. Si le fichier manque, la tache est bloquee (question Q&R).
- **Que faire d'un test avec un cycle `re` ou `we`?** Rien avant E11: le runner le compte comme `skipped_addr_error` (D-030).
- **Un test SingleStepTests echoue et le timing de Yacht.txt dit autre chose?** Ne devine pas: question Q&R avec les deux valeurs. Les tests viennent de MAME et peuvent contenir ses bugs (D-007).
- **Quel est le PC initial d'un test?** Adresse de l'instruction + 4 (prochain mot a prefetcher); IR et IRC sont dans `prefetch` (specs/33).
- **Quelle taille de tick?** Une horloge (D-002). Un cycle de bus = 4 horloges minimum.
- **Ou ecrire le code d'une instruction?** `crates/puce-m68k/src/insn/<mnemonique>.rs` + une ligne dans `insn/mod.rs`.
