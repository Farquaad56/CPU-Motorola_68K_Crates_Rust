# JMP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## JMP (lines 818-832)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       JMP        |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
<ea> :            |                 |
    (An)          |  8(2/0)         |                      np       np
    (d16,An)      | 10(2/0)         |                 n    np       np
    (d8,An,Xn)    | 14(2/0)         |                 n nn np       np
    (xxx).W       | 10(2/0)         |                 n    np       np
    (xxx).L       | 12(3/0)         |                   np np       np
NOTES :
  .M68000UM is probably wrong with bus read accesses for JMP (d8,An,Xn),Dn
   instruction. It reads "14(3/0)" but, according to USP4325121 and with a
   little common sense, 2 bus read accesses are far enough.
```

