# RTR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## RTE, RTR (lines 866-876)

```text
-------------------------------------------------------------------------------
	                |    Exec Time    |               Data Bus Usage
     RTE, RTR     |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
                  | 20(5/0)         |                      nU nu nu np np
NOTES :
  .M68000UM is probably wrong with bus read accesses for RTR instruction.
   It reads "20(2/0)" but, according to USP4325121 and with a little common
   sense, 2 bus read accesses aren't enough for reading status register and
   program counter values from the stack and prefetching.
```

