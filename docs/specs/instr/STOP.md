# STOP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## STOP (lines 857-865)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       STOP       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
                  |  4(0/0)         |                               n
NOTES :
  .At this point, there is no real clues of how the second microcyle of this *
   instruction is spent.
```

