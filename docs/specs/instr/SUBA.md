# SUBA

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1254-1257)

```text
*******************************************************************************
  Line 1001                             &    Line 1101
    SUB, SUBA, SUBX                            ADD, ADDA, ADDX
*******************************************************************************
```

## ADDA, SUBA (lines 1303-1330)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
    ADDA, SUBA    |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,An :         |                 |               |
  .B or .W :      |                 |               |
    Dn            |  8(1/0)  0(0/0) |               |               np       nn
    An            |  8(1/0)  0(0/0) |               |               np       nn
    (An)          |  8(1/0)  4(1/0) |            nr |               np       nn
    (An)+         |  8(1/0)  4(1/0) |            nr |               np       nn
    -(An)         |  8(1/0)  6(1/0) | n          nr |               np       nn
    (d16,An)      |  8(1/0)  8(2/0) |      np    nr |               np       nn
    (d8,An,Xn)    |  8(1/0) 10(2/0) | n    np    nr |               np       nn
    (xxx).W       |  8(1/0)  8(2/0) |      np    nr |               np       nn
    (xxx).L       |  8(1/0) 12(3/0) |   np np    nr |               np       nn
    #<data>       |  8(1/0)  4(1/0) |      np       |               np       nn
  .L :            |                 |               |
    Dn            |  8(1/0)  0(0/0) |               |               np       nn
    An            |  8(1/0)  0(0/0) |               |               np       nn
    (An)          |  6(1/0)  8(2/0) |         nR nr |               np       n
    (An)+         |  6(1/0)  8(2/0) |         nR nr |               np       n
    -(An)         |  6(1/0) 10(2/0) | n       nR nr |               np       n
    (d16,An)      |  6(1/0) 12(3/0) |      np nR nr |               np       n
    (d8,An,Xn)    |  6(1/0) 14(3/0) | n    np nR nr |               np       n
    (xxx).W       |  6(1/0) 12(3/0) |      np nR nr |               np       n
    (xxx).L       |  6(1/0) 16(4/0) |   np np nR nr |               np       n
    #<data>       |  8(1/0)  8(2/0) |   np np       |               np       nn
```

