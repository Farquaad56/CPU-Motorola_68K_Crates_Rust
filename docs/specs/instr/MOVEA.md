# MOVEA

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 335-338)

```text
*******************************************************************************
  Line 0001         &          Line 0010          &          Line 0011
    MOVE.B                       MOVE.L, MOVEA.L               MOVE.W, MOVEA.W
*******************************************************************************
```

## MOVEA (lines 529-559)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
      MOVEA       |      INSTR      |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,An :         |                 |               |
  .W :            |                 |               |
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
```

