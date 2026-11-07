# ADDX

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1254-1257)

```text
*******************************************************************************
  Line 1001                             &    Line 1101
    SUB, SUBA, SUBX                            ADD, ADDA, ADDX
*******************************************************************************
```

## ADDX, SUBX (lines 1331-1344)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
    ADDX, SUBX    |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
Dy,Dx :           |                 |
  .B or .W :      |  4(1/0)         |                               np
  .L :            |  8(1/0)         |                               np       nn
-(Ay),-(Ax) :     |                 |
  .B or .W :      | 18(3/1)         |              n nr    nr       np nw
  .L :            | 30(5/2)         |              n nr nR nr nR nw np    nW
```

