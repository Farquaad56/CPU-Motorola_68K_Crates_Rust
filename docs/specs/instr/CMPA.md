# CMPA

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1353-1356)

```text
*******************************************************************************
  Line 1011
    CMP, CMPA, CMPM, EOR
*******************************************************************************
```

## CMPA (lines 1385-1412)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       CMPA       |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,An :         |                 |               |
  .B or .W :      |                 |               |
    Dn            |  6(1/0)  0(0/0) |               |               np       n
    An            |  6(1/0)  0(0/0) |               |               np       n
    (An)          |  6(1/0)  4(1/0) |            nr |               np       n
    (An)+         |  6(1/0)  4(1/0) |            nr |               np       n
    -(An)         |  6(1/0)  6(1/0) | n          nr |               np       n
    (d16,An)      |  6(1/0)  8(2/0) |      np    nr |               np       n
    (d8,An,Xn)    |  6(1/0) 10(2/0) | n    np    nr |               np       n
    (xxx).W       |  6(1/0)  8(2/0) |      np    nr |               np       n
    (xxx).L       |  6(1/0) 12(3/0) |   np np    nr |               np       n
    #<data>       |  6(1/0)  4(1/0) |      np       |               np       n
  .L :            |                 |               |
    Dn            |  6(1/0)  0(0/0) |               |               np       n
    An            |  6(1/0)  0(0/0) |               |               np       n
    (An)          |  6(1/0)  8(1/0) |         nR nr |               np       n
    (An)+         |  6(1/0)  8(1/0) |         nR nr |               np       n
    -(An)         |  6(1/0) 10(1/0) | n       nR nr |               np       n
    (d16,An)      |  6(1/0) 12(2/0) |      np nR nr |               np       n
    (d8,An,Xn)    |  6(1/0) 14(2/0) | n    np nR nr |               np       n
    (xxx).W       |  6(1/0) 12(2/0) |      np nR nr |               np       n
    (xxx).L       |  6(1/0) 16(3/0) |   np np nR nr |               np       n
    #<data>       |  6(1/0)  8(2/0) |   np np       |               np       n
```

