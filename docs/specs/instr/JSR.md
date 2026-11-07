# JSR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## JSR (lines 807-817)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       JSR        |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
<ea> :            |                 |
    (An)          | 16(2/2)         |                      np nS ns np
    (d16,An)      | 18(2/2)         |                 n    np nS ns np
    (d8,An,Xn)    | 22(2/2)         |                 n nn np nS ns np
    (xxx).W       | 18(2/2)         |                 n    np nS ns np
    (xxx).L       | 20(3/2)         |                   np np nS ns np
```

