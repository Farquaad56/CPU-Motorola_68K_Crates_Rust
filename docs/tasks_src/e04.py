from _dsl import T

T("E04-01", "GATE - telecharger Yacht.txt et les manuels", gate=True, kind="gate", size=0,
  context="Les trois documents de reference ne sont pas telechargeables depuis l'environnement de Claude (hotes bloques): l'utilisateur les telecharge une fois. Les tests SingleStepTests et les sources de la ROM opcode-sizes sont deja presents.",
  user_cmds=["python docs/tools/fetch_refs.py --only docs", "dir docs\\ref"],
  accept=[])

T("E04-02", "Decouper Yacht.txt en un fichier par instruction", kind="tooling", size=150,
  context="Les cartes d'instruction embarquent leur spec de timing: il faut un fichier `docs/specs/instr/<MNEMONIQUE>.md` par instruction (contexte LLM 135000 tokens: un fichier = quelques centaines de lignes max). Le format de Yacht.txt n'est pas connu a l'avance: lis-le d'abord.",
  reads=["`docs/ref/Yacht.txt` (a lire en entier avant d'ecrire le script)", "`docs/specs/data/instructions.tsv` (liste des mnemoniques attendus, colonne `mnemonic`)"],
  spec="""Cree `docs/tools/split_yacht.py` (Python 3.8+, bibliotheque standard): decoupe Yacht.txt en `docs/specs/instr/<MNEMONIQUE>.md` (un fichier par mnemonique de la colonne `mnemonic`, y compris les variantes de taille dans le meme fichier), en gardant le texte original (copie fidele, avec un en-tete `# <MNEMONIQUE>` et la source `docs/ref/Yacht.txt` + numeros de lignes).
Options: `--check` verifie que chaque mnemonique de `instructions.tsv` a un fichier non vide et liste les manquants; sans option, regenere tout.
Les mnemoniques absents de Yacht.txt (ex. exceptions) sont listes dans `docs/specs/instr/MISSING.txt` avec la raison; ne devine rien.""",
  write=["docs/tools/split_yacht.py", "docs/specs/instr/**"],
  accept=["python docs/tools/split_yacht.py", "python docs/tools/split_yacht.py --check"])

T("E04-03", "Tableau des modes d'adressage (timing)", kind="doc", size=0,
  context="Les plans d'adresse effective (E05) ont besoin du timing de chaque mode d'adressage. Extrait-le de Yacht.txt et du manuel (chapitre 8, tableaux de calcul d'adresse effective).",
  reads=["`docs/ref/Yacht.txt`", "`docs/ref/M68000UM_Rev8.pdf` (si pdftotext disponible: `pdftotext -layout`; sinon demande a l'utilisateur le texte du chapitre 8)"],
  spec="""Ecris `docs/specs/instr/_EA.md`: pour chacun des 12 modes (Dn, An, (An), (An)+, -(An), d16(An), d8(An,Xn), abs.W, abs.L, d16(PC), d8(PC,Xn), #imm) et pour les tailles octet/mot et long: nombre d'horloges ajoutees, nombre de lectures/ecritures, sequence des cycles de bus (extension word = prefetch, cycles internes), en indiquant la source (fichier et ligne) pour chaque valeur. Valeur introuvable: `UNKNOWN - to confirm` + question Q&R.""",
  write=["docs/specs/instr/_EA.md"], accept=["python docs/tools/split_yacht.py --check"])

T("E04-04", "GATE E04 - specs par instruction", gate=True, kind="gate", size=0,
  context="Validation humaine: les specs extraites sont fideles au texte source et couvrent les 127 fichiers de tests.",
  user_cmds=["python docs/tools/split_yacht.py --check", "type docs\\specs\\instr\\NOP.md", "type docs\\specs\\instr\\_EA.md"],
  accept=[])
