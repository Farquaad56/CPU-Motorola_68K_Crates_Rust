# TAS

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## TAS (lines 732-752)

```text
-------------------------------------------------------------------------------
	                |    Exec Time    |               Data Bus Usage
       TAS        |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea> :            |                 |               |
  .B :            |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    (An)          | 10(1/1)  4(1/0) |            nr |          n nw np
    (An)+         | 10(1/1)  4(1/0) |            nr |          n nw np
    -(An)         | 10(1/1)  6(1/0) | n          nr |          n nw np
    (d16,An)      | 10(1/1)  8(2/0) |      np    nr |          n nw np
    (d8,An,Xn)    | 10(1/1) 10(2/0) | n    np    nr |          n nw np
    (xxx).W       | 10(1/1)  8(1/0) |               |          n nw np
    (xxx).L       | 10(1/1) 12(2/0) |               |          n nw np
NOTES :
  .M68000UM is probably wrong with instruction timming when <ea> is different
   from Dn. It reads "14(3/0)" but according to the same book the read-modify-
   write bus cycle used by this instruction is only 10 clock cycles long which
   seems coherent with the microwords decomposition in USP4325121. Last,
   evaluation on real hardware confirms the 10 cycles timing.
```

