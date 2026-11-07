# NOP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## NOP (lines 851-856)

```text
-------------------------------------------------------------------------------
	                |    Exec Time    |               Data Bus Usage
       NOP        |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
                  |  4(1/0)         |                               np
```

