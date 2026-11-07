# MOVEM

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## MOVEM (lines 655-699)

```text
-------------------------------------------------------------------------------
	                |    Exec Time    |               Data Bus Usage
      MOVEM       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
M --> R           |                 |
  .W              |                 |
    (An)          | 12+4m(3+m/0)    |                np (nr)*    nr np
    (An)+         | 12+4m(3+m/0)    |                np (nr)*    nr np
    (d16,An)      | 16+4m(4+m/0)    |             np np (nr)*    nr np
    (d8,An,Xn)    | 18+4m(4+m/0)    |          np n  np (nr)*    nr np
    (xxx).W       | 16+4m(4+m/0)    |             np np (nr)*    nr np
    (xxx).L       | 20+4m(5+m/0)    |          np np np (nr)*    nr np
  .L              |                 |
    (An)          | 12+8m(3+2m/0)   |                np nR (nr nR)* np
    (An)+         | 12+8m(3+2m/0)   |                np nR (nr nR)* np
    (d16,An)      | 16+8m(4+2m/0)   |             np np nR (nr nR)* np
    (d8,An,Xn)    | 18+8m(4+2m/0)   |          np n  np nR (nr nR)* np
    (xxx).W       | 16+8m(4+2m/0)   |             np np nR (nr nR)* np
    (xxx).L       | 20+8m(5+2m/0)   |          np np np nR (nr nR)* np
R --> M           |                 |
  .W              |                 |
    (An)          |  8+4m(2/m)      |                np (nw)*       np
    -(An)         |  8+4m(2/m)      |                np (nw)*       np
    (d16,An)      | 12+4m(3/m)      |             np np (nw)*       np
    (d8,An,Xn)    | 14+4m(3/m)      |          np n  np (nw)*       np
    (xxx).W       | 12+4m(3/m)      |             np np (nw)*       np
    (xxx).L       | 16+4m(4/m)      |          np np np (nw)*       np
  .L              |                 |
    (An)          |  8+8m(2/2m)     |                np (nW nw)*    np
    -(An)         |  8+8m(2/2m)     |                np (nw nW)*    np
    (d16,An)      | 12+8m(3/2m)     |             np np (nW nw)*    np
    (d8,An,Xn)    | 14+8m(3/2m)     |          np n  np (nW nw)*    np
    (xxx).W       | 12+8m(3/2m)     |             np np (nW nw)*    np
    (xxx).L       | 16+8m(4/2m)     |          np np np (nW nw)*    np
NOTES :
  .'m' is the number of registers to move.
  .'(nr)*' should be replaced by m consecutive 'nr'
  .'(nw)*' should be replaced by m consecutive 'nw'
  .'(nR nr)*' should be replaced by m consecutive 'nR nr'
  .'(nW nw)*' (or '(nw nW)*') should be replaced by m consecutive 'nW nw' (or m
   consecutive 'nw nW').
  .In M --> R mode, an extra bus read cycle occurs. This extra access can cause
   errors (for exemple if occuring beyond the bounds of physical memory).
  .In M --> R mode, MOVEM.W sign-extends words moved to data registers.
```

