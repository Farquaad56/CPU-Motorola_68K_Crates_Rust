# LEA

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## LEA (lines 795-806)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        LEA       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
<ea>,An :         |                 |
  .L :            |                 |
    (An)          |  4(1/0)         |                               np
    (d16,An)      |  8(2/0)         |                          np   np
    (d8,An,Xn)    | 12(2/0)         |                        n np n np
    (xxx).W       |  8(2/0)         |                          np   np
    (xxx).L       | 12(3/0)         |                       np np   np
```

