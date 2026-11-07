# TST

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## TST (lines 708-731)

```text
-------------------------------------------------------------------------------
	                |     Exec Time   |               Data Bus Usage
       TST        |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea> :            |                 |               |
  .B or .W :      |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    (An)          |  4(1/0)  4(1/0) |            nr |               np
    (An)+         |  4(1/0)  4(1/0) |            nr |               np
    -(An)         |  4(1/0)  6(1/0) | n          nr |               np
    (d16,An)      |  4(1/0)  8(2/0) |      np    nr |               np
    (d8,An,Xn)    |  4(1/0) 10(2/0) | n    np    nr |               np
    (xxx).W       |  4(1/0)  8(2/0) |      np    nr |               np
    (xxx).L       |  4(1/0) 12(3/0) |   np np    nr |               np
  .L              |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    (An)          |  4(1/0)  8(2/0) |         nR nr |               np
    (An)+         |  4(1/0)  8(2/0) |         nR nr |               np
    -(An)         |  4(1/0) 10(2/0) | n       nR nr |               np
    (d16,An)      |  4(1/0) 12(3/0) |      np nR nr |               np
    (d8,An,Xn)    |  4(1/0) 14(3/0) | n    np nR nr |               np
    (xxx).W       |  4(1/0) 12(3/0) |      np nR nr |               np
    (xxx).L       |  4(1/0) 16(4/0) |   np np nR nr |               np
```

