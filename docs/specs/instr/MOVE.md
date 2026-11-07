# MOVE

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 335-338)

```text
*******************************************************************************
  Line 0001         &          Line 0010          &          Line 0011
    MOVE.B                       MOVE.L, MOVEA.L               MOVE.W, MOVEA.W
*******************************************************************************
```

## MOVE (lines 340-528)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       MOVE       |      INSTR      |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,Dn :         |                 |               |
  .B or .W :      |                 |               |
    Dn            |  4(1/0)         |               |               np
    An            |  4(1/0)         |               |               np
    (An)          |  8(2/0)         |            nr |               np
    (An)+         |  8(2/0)         |            nr |               np
    -(An)         | 10(2/0)         | n          nr |               np
    (d16,An)      | 12(3/0)         |      np    nr |               np
    (d8,An,Xn)    | 14(3/0)         | n    np    nr |               np
    (xxx).W       | 12(3/0)         |      np    nr |               np
    (xxx).L       | 16(4/0)         |   np np    nr |               np
    #<data>       |  8(2/0)         |      np       |               np
  .L :            |                 |               |
    Dn            |  4(1/0)         |               |               np
    An            |  4(1/0)         |               |               np
    (An)          | 12(3/0)         |         nR nr |               np
    (An)+         | 12(3/0)         |         nR nr |               np
    -(An)         | 14(3/0)         | n       nR nr |               np
    (d16,An)      | 16(4/0)         |      np nR nr |               np
    (d8,An,Xn)    | 18(4/0)         | n    np nR nr |               np
    (xxx).W       | 16(4/0)         |      np nR nr |               np
    (xxx).L       | 20(5/0)         |   np np nR nr |               np
    #<data>       | 12(3/0)         |   np np       |               np
<ea>,(An) :       |                 |               |
  .B or .W :      |                 |               |
    Dn            |  8(1/1)         |               |            nw np
    An            |  8(1/1)         |               |            nw np
    (An)          | 12(2/1)         |            nr |            nw np
    (An)+         | 12(2/1)         |            nr |            nw np
    -(An)         | 14(2/1)         | n          nr |            nw np
    (d16,An)      | 16(3/1)         |      np    nr |            nw np
    (d8,An,Xn)    | 18(3/1)         | n    np    nr |            nw np
    (xxx).W       | 16(3/1)         |      np    nr |            nw np
    (xxx).L       | 20(4/1)         |   np np    nr |            nw np
    #<data>       | 12(2/1)         |      np       |            nw np
  .L :            |                 |               |
    Dn            | 12(1/2)         |               |         nW nw np
    An            | 12(1/2)         |               |         nW nw np
    (An)          | 20(3/2)         |         nR nr |         nW nw np
    (An)+         | 20(3/2)         |         nR nr |         nW nw np
    -(An)         | 22(3/2)         | n       nR nr |         nW nw np
    (d16,An)      | 24(4/2)         |      np nR nr |         nW nw np
    (d8,An,Xn)    | 26(4/2)         | n    np nR nr |         nW nw np
    (xxx).W       | 24(4/2)         |      np nR nr |         nW nw np
    (xxx).L       | 28(5/2)         |   np np nR nr |         nW nw np
    #<data>       | 20(3/2)         |   np np       |         nW nw np
<ea>,(An)+ :      |                 |               |
  .B or .W :      |                 |               |
    Dn            |  8(1/1)         |               |            nw np
    An            |  8(1/1)         |               |            nw np
    (An)          | 12(2/1)         |            nr |            nw np
    (An)+         | 12(2/1)         |            nr |            nw np
    -(An)         | 14(2/1)         | n          nr |            nw np
    (d16,An)      | 16(3/1)         |      np    nr |            nw np
    (d8,An,Xn)    | 18(3/1)         | n    np    nr |            nw np
    (xxx).W       | 16(3/1)         |      np    nr |            nw np
    (xxx).L       | 20(4/1)         |   np np    nr |            nw np
    #<data>       | 12(2/1)         |      np       |            nw np
  .L :            |                 |               |
    Dn            | 12(1/2)         |               |         nW nw np
    An            | 12(1/2)         |               |         nW nw np
    (An)          | 20(3/2)         |         nR nr |         nW nw np
    (An)+         | 20(3/2)         |         nR nr |         nW nw np
    -(An)         | 22(3/2)         | n       nR nr |         nW nw np
    (d16,An)      | 24(4/2)         |      np nR nr |         nW nw np
    (d8,An,Xn)    | 26(4/2)         | n    np nR nr |         nW nw np
    (xxx).W       | 24(4/2)         |      np nR nr |         nW nw np
    (xxx).L       | 28(5/2)         |   np np nR nr |         nW nw np
    #<data>       | 20(3/2)         |   np np       |         nW nw np
<ea>,-(An) :      |                 |               |
  .B or .W :      |                 |               |
    Dn            |  8(1/1)         |               |                  np nw
    An            |  8(1/1)         |               |                  np nw
    (An)          | 12(2/1)         |            nr |                  np nw
    (An)+         | 12(2/1)         |            nr |                  np nw
    -(An)         | 14(2/1)         | n          nr |                  np nw
    (d16,An)      | 16(3/1)         |      np    nr |                  np nw
    (d8,An,Xn)    | 18(3/1)         | n    np    nr |                  np nw
    (xxx).W       | 16(3/1)         |      np    nr |                  np nw
    (xxx).L       | 20(4/1)         |   np np    nr |                  np nw
    #<data>       | 12(2/1)         |      np       |                  np nw
  .L :            |                 |               |
    Dn            | 12(1/2)         |               |                  np nw nW
    An            | 12(1/2)         |               |                  np nw nW
    (An)          | 20(3/2)         |         nR nr |                  np nw nW
    (An)+         | 20(3/2)         |         nR nr |                  np nw nW
    -(An)         | 22(3/2)         | n       nR nr |                  np nw nW
    (d16,An)      | 24(4/2)         |      np nR nr |                  np nw nW
    (d8,An,Xn)    | 26(4/2)         | n    np nR nr |                  np nw nW
    (xxx).W       | 24(4/2)         |      np nR nr |                  np nw nW
    (xxx).L       | 28(5/2)         |   np np nR nr |                  np nw nW
    #<data>       | 20(3/2)         |   np np       |                  np nw nW
<ea>,(d16,An) :   |                 |               |
  .B or .W :      |                 |               |
    Dn            | 12(2/1)         |               |      np    nw np
    An            | 12(2/1)         |               |      np    nw np
    (An)          | 16(3/1)         |            nr |      np    nw np
    (An)+         | 16(3/1)         |            nr |      np    nw np
    -(An)         | 18(3/1)         | n          nr |      np    nw np
    (d16,An)      | 20(4/1)         |      np    nr |      np    nw np
    (d8,An,Xn)    | 22(4/1)         | n    np    nr |      np    nw np
    (xxx).W       | 20(4/1)         |      np    nr |      np    nw np
    (xxx).L       | 24(5/1)         |   np np    nr |      np    nw np
    #<data>       | 16(3/1)         |      np       |      np    nw np
  .L :            |                 |               |
    Dn            | 16(2/2)         |               |      np nW nw np
    An            | 16(2/2)         |               |      np nW nw np
    (An)          | 24(4/2)         |         nR nr |      np nW nw np
    (An)+         | 24(4/2)         |         nR nr |      np nW nw np
    -(An)         | 26(4/2)         | n       nR nr |      np nW nw np
    (d16,An)      | 28(5/2)         |      np nR nr |      np nW nw np
    (d8,An,Xn)    | 30(5/2)         | n    np nR nr |      np nW nw np
    (xxx).W       | 28(5/2)         |      np nR nr |      np nW nw np
    (xxx).L       | 32(6/2)         |   np np nR nr |      np nW nw np
    #<data>       | 24(4/2)         |   np np       |      np nW nw np
<ea>,(d8,An,Xn) : |                 |               |
  .B or .W :      |                 |               |
    Dn            | 14(2/1)         |               | n    np    nw np
    An            | 14(2/1)         |               | n    np    nw np
    (An)          | 18(3/1)         |            nr | n    np    nw np
    (An)+         | 18(3/1)         |            nr | n    np    nw np
    -(An)         | 20(3/1)         | n          nr | n    np    nw np
    (d16,An)      | 22(4/1)         |      np    nr | n    np    nw np
    (d8,An,Xn)    | 24(4/1)         | n    np    nr | n    np    nw np
    (xxx).W       | 22(4/1)         |      np    nr | n    np    nw np
    (xxx).L       | 26(5/1)         |   np np    nr | n    np    nw np
    #<data>       | 18(3/1)         |      np       | n    np    nw np
  .L :            |                 |               |
    Dn            | 18(2/2)         |               | n    np nW nw np
    An            | 18(2/2)         |               | n    np nW nw np
    (An)          | 26(4/2)         |         nR nr | n    np nW nw np
    (An)+         | 26(4/2)         |         nR nr | n    np nW nw np
    -(An)         | 28(4/2)         | n       nR nr | n    np nW nw np
    (d16,An)      | 30(5/2)         |      np nR nr | n    np nW nw np
    (d8,An,Xn)    | 32(5/2)         | n    np nR nr | n    np nW nw np
    (xxx).W       | 30(5/2)         |      np nR nr | n    np nW nw np
    (xxx).L       | 34(6/2)         |   np np nR nr | n    np nW nw np
    #<data>       | 26(4/2)         |   np np       | n    np nW nw np
<ea>,(xxx).W :    |                 |               |
  .B or .W :      |                 |               |
    Dn            | 12(2/1)         |               |      np    nw np
    An            | 12(2/1)         |               |      np    nw np
    (An)          | 16(3/1)         |            nr |      np    nw np
    (An)+         | 16(3/1)         |            nr |      np    nw np
    -(An)         | 18(3/1)         | n          nr |      np    nw np
    (d16,An)      | 20(4/1)         |      np    nr |      np    nw np
    (d8,An,Xn)    | 22(4/1)         | n    np    nr |      np    nw np
    (xxx).W       | 20(4/1)         |      np    nr |      np    nw np
    (xxx).L       | 24(5/1)         |   np np    nr |      np    nw np
    #<data>       | 16(3/1)         |      np       |      np    nw np
  .L :            |                 |               |
    Dn            | 16(2/2)         |               |      np nW nw np
    An            | 16(2/2)         |               |      np nW nw np
    (An)          | 24(4/2)         |         nR nr |      np nW nw np
    (An)+         | 24(4/2)         |         nR nr |      np nW nw np
    -(An)         | 26(4/2)         | n       nR nr |      np nW nw np
    (d16,An)      | 28(5/2)         |      np nR nr |      np nW nw np
    (d8,An,Xn)    | 30(5/2)         | n    np nR nr |      np nW nw np
    (xxx).W       | 28(5/2)         |      np nR nr |      np nW nw np
    (xxx).L       | 32(6/2)         |   np np nR nr |      np nW nw np
    #<data>       | 24(4/2)         |   np np       |      np nW nw np
<ea>,(xxx).L :    |                 |               |
  .B or .W :      |                 |               |
    Dn            | 16(3/1)         |               |   np np    nw np
    An            | 16(3/1)         |               |   np np    nw np
    (An)          | 20(4/1)         |            nr |      np    nw np np
    (An)+         | 20(4/1)         |            nr |      np    nw np np
    -(An)         | 22(4/1)         | n          nr |      np    nw np np
    (d16,An)      | 24(5/1)         |      np    nr |      np    nw np np
    (d8,An,Xn)    | 26(5/1)         | n    np    nr |      np    nw np np
    (xxx).W       | 24(5/1)         |      np    nr |      np    nw np np
    (xxx).L       | 28(6/1)         |   np np    nr |      np    nw np np
    #<data>       | 20(4/1)         |      np       |   np np    nw np
  .L :            |                 |               |
    Dn            | 20(3/2)         |               |   np np nW nw np
    An            | 20(3/2)         |               |   np np nW nw np
    (An)          | 28(5/2)         |         nR nr |      np nW nw np np
    (An)+         | 28(5/2)         |         nR nr |      np nW nw np np
    -(An)         | 30(5/2)         | n       nR nr |      np nW nw np np
    (d16,An)      | 32(6/2)         |      np nR nr |      np nW nw np np
    (d8,An,Xn)    | 34(6/2)         | n    np nR nr |      np nW nw np np
    (xxx).W       | 32(6/2)         |      np nR nr |      np nW nw np np
    (xxx).L       | 36(7/2)         |   np np nR nr |      np nW nw np np
    #<data>       | 28(5/2)         |   np np       |   np np nW nw np
```

