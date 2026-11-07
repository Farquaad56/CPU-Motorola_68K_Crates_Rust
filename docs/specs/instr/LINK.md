# LINK

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## LINK (lines 892-898)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       LINK       |      INSTR      |  2nd Operand  |   INSTR
------------------+-----------------+---------------+--------------------------
An,#<data> :      |                 |
  .W :            | 16(2/2)         |                      np nS ns np
```

