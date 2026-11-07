# NOT

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## CLR, NEGX, NEG, NOT (lines 566-589)

```text
-------------------------------------------------------------------------------
        CLR,      |    Exec Time    |               Data Bus Usage
  NEGX, NEG, NOT  |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea> :            |                 |               |
  .B or .W :      |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    (An)          |  8(1/1)  4(1/0) |            nr |               np nw
    (An)+         |  8(1/1)  4(1/0) |            nr |               np nw
    -(An)         |  8(1/1)  6(1/0) | n          nr |               np nw
    (d16,An)      |  8(1/1)  8(2/0) |      np    nr |               np nw
    (d8,An,Xn)    |  8(1/1) 10(2/0) | n    np    nr |               np nw
    (xxx).W       |  8(1/1)  8(2/0) |      np    nr |               np nw
    (xxx).L       |  8(1/1) 12(2/0) |   np np    nr |               np nw
  .L :            |                 |               |
    Dn            |  6(1/0)  0(0/0) |               |               np       n
    (An)          | 12(1/2)  8(2/0) |         nR nr |               np nw nW
    (An)+         | 12(1/2)  8(2/0) |         nR nr |               np nw nW
    -(An)         | 12(1/2) 10(2/0) | n       nR nr |               np nw nW
    (d16,An)      | 12(1/2) 12(3/0) |      np nR nr |               np nw nW
    (d8,An,Xn)    | 12(1/2) 14(3/0) | n    np nR nr |               np nw nW
    (xxx).W       | 12(1/2) 12(3/0) |      np nR nr |               np nw nW
    (xxx).L       | 12(1/2) 16(4/0) |   np np nR nr |               np nw nW
```

