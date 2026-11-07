# PEA

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## PEA (lines 636-647)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        PEA       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
<ea> :            |                 |
  .L :            |                 |
    (An)          | 12(1/2)         |                               np nS ns
    (d16,An)      | 16(2/2)         |                          np   np nS ns
    (d8,An,Xn)    | 20(2/2)         |                        n np n np nS ns
    (xxx).W       | 16(2/2)         |                               np nS ns np
    (xxx).L       | 20(3/2)         |                          np   np nS ns np
```

