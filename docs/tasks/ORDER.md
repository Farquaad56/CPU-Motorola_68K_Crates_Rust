# ORDER - ordre d'execution strict (genere)

D-020: UNE seule tache a la fois, dans cet ordre. Une tache n'est prete que quand la precedente est `done` (et approuvee si GATE).


## E01
  1. E01-01 Squelette du crate puce-m68k
  2. E01-02 Squelette du crate puce-m68k-machines
  3. E01-03 Squelette du binaire puce-m68k-cli (m68k)
  4. E01-04 Squelette du binaire puce-m68k-web (m68k-web)
  5. E01-05 Squelette du crate puce-m68k-wasm
  6. E01-06 Scripts de CI et d'hygiene
  7. E01-07 GATE E01 - workspace valide  [GATE]

## E02
  8. E02-01 Type Pins (broches du 68000)
  9. E02-02 Registres (Regs)
 10. E02-03 Trait Bus de confort
 11. E02-04 Jetons de plan (Tok, Plan)
 12. E02-05 Profil des broches par horloge (cycle de bus)
 13. E02-06 Moteur: execution d'un plan avec attente DTACK
 14. E02-07 GATE E02 - broches, registres, bus et moteur de base  [GATE]

## E03
 15. E03-01 Lecteur du format SingleStepTests (.json.bin)
 16. E03-02 Machine RAM plate (FlatRam)
 17. E03-03 Etat du CPU: load_state, state, tick_bus
 18. E03-04 Journal de bus (feature inspect)
 19. E03-05 Runner: execution et comparaison d'un test
 20. E03-06 Runner: fichiers, repertoire, exceptions connues
 21. E03-07 Commande m68k ssc
 22. E03-08 GATE E03 - runner valide (tous les tests echouent: attendu)  [GATE]

## E04
 23. E04-01 GATE - telecharger Yacht.txt et les manuels  [GATE]
 24. E04-02 Decouper Yacht.txt en un fichier par instruction
 25. E04-03 Tableau des modes d'adressage (timing)
 26. E04-04 GATE E04 - specs par instruction  [GATE]

## E05
 27. E05-01 Table de decodage (65536 entrees, const fn)
 28. E05-02 Champs d'adressage effectif et categories de validite
 29. E05-03 File de prefetch (IR/IRC) et jeton Pf
 30. E05-04a Plans d'adresse effective: Dn, An, (An), (An)+, -(An)
 31. E05-04b Plans d'adresse effective: d16(An), d8(An,Xn), abs, PC-relatif, immediat
 32. E05-05 Boucle d'instruction: decodage et execution (step)
 33. E05-06 GATE E05 - decodage, EA, prefetch, step  [GATE]

## E06
 34. E06-01 NOP
 35. E06-02a MOVE (1/2): sources registre et immediat
 36. E06-02b MOVE (2/2): tous les autres modes
 37. E06-03 MOVEA
 38. E06-04 MOVEQ
 39. E06-05a MOVEM (1/2): registres vers memoire
 40. E06-05b MOVEM (2/2): memoire vers registres
 41. E06-06 MOVEP
 42. E06-07 LEA
 43. E06-08 PEA
 44. E06-09 EXG
 45. E06-10 SWAP
 46. E06-11 EXT
 47. E06-12 LINK
 48. E06-13 UNLINK
 49. E06-14 CLR
 50. E06-99 GATE E06 - transferts de donnees  [GATE]

## E07
 51. E07-01 ADD
 52. E07-02 ADDA
 53. E07-03 ADDX
 54. E07-04 SUB
 55. E07-05 SUBA
 56. E07-06 SUBX
 57. E07-07 NEG
 58. E07-08 NEGX
 59. E07-09 CMP
 60. E07-10 CMPA
 61. E07-11 TST
 62. E07-12 MULU
 63. E07-13 MULS
 64. E07-14a DIVU (1/2): quotient et reste
 65. E07-14b DIVU (2/2): timing de l'algorithme
 66. E07-15a DIVS (1/2): quotient et reste signes
 67. E07-15b DIVS (2/2): timing de l'algorithme
 68. E07-99 GATE E07 - arithmetique entiere  [GATE]

## E08
 69. E08-01 AND
 70. E08-02 OR
 71. E08-03 EOR
 72. E08-04 NOT
 73. E08-05 ASL
 74. E08-06 ASR
 75. E08-07 LSL
 76. E08-08 LSR
 77. E08-09 ROL
 78. E08-10 ROR
 79. E08-11 ROXL
 80. E08-12 ROXR
 81. E08-13 BTST
 82. E08-14 BSET
 83. E08-15 BCLR
 84. E08-16 BCHG
 85. E08-17 ABCD
 86. E08-18 SBCD
 87. E08-19 NBCD
 88. E08-99 GATE E08 - logique, decalages, bits et BCD  [GATE]

## E09
 89. E09-01 Bcc
 90. E09-02 BSR
 91. E09-03 DBcc
 92. E09-04 Scc
 93. E09-05 JMP
 94. E09-06 JSR
 95. E09-07 RTS
 96. E09-08 RTR
 97. E09-99 GATE E09 - controle de flux  [GATE]

## E10
 98. E10-01 ANDItoCCR
 99. E10-02 ANDItoSR
100. E10-03 ORItoCCR
101. E10-04 ORItoSR
102. E10-05 EORItoCCR
103. E10-06 EORItoSR
104. E10-07 MOVEtoCCR
105. E10-08 MOVEtoSR
106. E10-09 MOVEfromSR
107. E10-10 MOVEtoUSP
108. E10-11 MOVEfromUSP
109. E10-12 TAS
110. E10-99 GATE E10 - registres d'etat et divers  [GATE]

## E11
111. E11-01 GATE E11 - demarrage des exceptions et interruptions  [GATE]
112. E11-02 Traitement d'exception (groupes 1 et 2) et vecteurs
113. E11-03 Address error (frame de groupe 0)
114. E11-04 Violation de privilege, opcode illegal, lignes A et F
115. E11-05 Interruptions IPL0-2 et autovecteur
116. E11-06 RESET, HALT et double faute de bus
117. E11-07 TRAP
118. E11-08 TRAPV
119. E11-09 CHK
120. E11-10 ILLEGAL_LINEA
121. E11-11 ILLEGAL_LINEF
122. E11-12 RTE
123. E11-13 STOP
124. E11-14 RESET

## E12
125. E12-01 Desassembleur: infrastructure et formatage des operandes
126. E12-02 Desassembleur: transferts et arithmetique
127. E12-03 Desassembleur: logique, decalages, bits, BCD
128. E12-04 Desassembleur: controle de flux et systeme
129. E12-05 Desassembleur: couverture des 65536 opcodes
130. E12-06 CLI: commandes run et disasm
131. E12-07 CLI: commande fetch
132. E12-08 GATE E12 - desassembleur et CLI  [GATE]

## E13
133. E13-01 Session: commandes et boucle de simulation
134. E13-02 Session: instantane d'etat conforme au protocole
135. E13-03 Serveur HTTP: page et sante
136. E13-04 Serveur WebSocket (/ws)
137. E13-05 Commandes: roms, load, upload, sst
138. E13-06 Page web Tailwind: panneaux et protocole
139. E13-07 Test bout en bout du serveur (client Rust)
140. E13-08 Binaire m68k-web: demarrage et arret propre
141. E13-09 GATE E13 - serveur et page valides  [GATE]

## E14
142. E14-01 Liaison wasm-bindgen du coeur
143. E14-02 Demo WebAssembly (page statique Tailwind)
144. E14-03 GATE E14 - demo WASM  [GATE]

## E15
145. E15-01 Performance du coeur (release)
146. E15-02 Baseline complete SingleStepTests
147. E15-03 Documentation du crate et README final
148. E15-04 GATE FINAL - crate 68000 complet  [GATE]
