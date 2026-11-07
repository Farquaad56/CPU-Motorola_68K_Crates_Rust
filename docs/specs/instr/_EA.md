# _EA - Effective address timing (68000)

Source: `docs/ref/Yacht.txt` lines 131-155 (table "OPERAND EFFECTIVE ADDRESS CALCULATION TIMES"). Format `n(r/w)` = clocks added (reads/writes).
Each value below is copied from the cited line. Nothing is guessed: what is not in Yacht.txt is marked `UNKNOWN - to confirm`.

## How to read the bus sequence (Yacht.txt lines 61-86, 91-94)

- A microcycle is 2 clocks; a bus cycle is 4 clocks (Yacht.txt vocabulary).
- Each token of the sequence is one microcycle (2 clocks); spaces and pipes carry no meaning.
  The sequence `nr` is therefore one 4-clock bus read (n + r), `np` one 4-clock program fetch, and a lone `n` is a 2-clock internal wait.
  Check: -(An).B/W = `n` + `nr` = 2 + 4 = 6 clocks (line 139). This reading is consistent with every row; to confirm in E05 against the SST tests.
- `p` = program fetch (extension word, goes into IRC), `r` = read word (LSW for long), `R` = read MSW of a long.

## Bytes and words (.B / .W)

| Mode | Clocks n(r/w) | Bus sequence | Source line |
|---|---|---|---|
| Dn | 0(0/0) | - | 135 |
| An | 0(0/0) | - | 136 |
| (An) | 4(1/0) | nr | 137 |
| (An)+ | 4(1/0) | nr | 138 |
| -(An) | 6(1/0) | n nr | 139 |
| (d16,An) | 8(2/0) | np nr | 140 |
| (d8,An,Xn) | 10(2/0) | n np nr | 141 |
| (xxx).W | 8(2/0) | np nr | 142 |
| (xxx).L | 12(3/0) | np np nr | 143 |
| #<data> | 4(1/0) | np | 144 |
| (d16,PC) | UNKNOWN - to confirm | UNKNOWN - to confirm | not in Yacht.txt |
| (d8,PC,Xn) | UNKNOWN - to confirm | UNKNOWN - to confirm | not in Yacht.txt |

## Long (.L)

| Mode | Clocks n(r/w) | Bus sequence | Source line |
|---|---|---|---|
| Dn | 0(0/0) | - | 146 |
| An | 0(0/0) | - | 147 |
| (An) | 8(2/0) | nR nr | 148 |
| (An)+ | 8(2/0) | nR nr | 149 |
| -(An) | 10(2/0) | n nR nr | 150 |
| (d16,An) | 12(3/0) | np nR nr | 151 |
| (d8,An,Xn) | 14(3/0) | n np nR nr | 152 |
| (xxx).W | 12(3/0) | np nR nr | 153 |
| (xxx).L | 16(4/0) | np np nR nr | 154 |
| #<data> | 8(2/0) | np np | 155 |
| (d16,PC) | UNKNOWN - to confirm | UNKNOWN - to confirm | not in Yacht.txt |
| (d8,PC,Xn) | UNKNOWN - to confirm | UNKNOWN - to confirm | not in Yacht.txt |

## Notes

- Yacht.txt rule 5 (line 59-60): (d16,SP) = (d16,An) and (d8,SP,Xn) = (d8,An,Xn) (A7 = SP).
- The PC-relative modes do not appear in this table. The M68000UM chapter 8 table (docs/ref/M68000UM_Rev8.pdf) was not read here (no text extraction done); question `docs/qa/inbox/E04-03-Q1.md`.
- For -(An) the `n` comes BEFORE the read (predecrement internal cycle, 2 clocks).
- These are the EA parts only: the "INSTR" part of every instruction is in `docs/specs/instr/<MNEMONIC>.md`; add both for the total, and the order of the sequences is given per instruction table (columns 1st operand / 2nd operand / INSTR).
