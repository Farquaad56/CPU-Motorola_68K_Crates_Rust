# EXG

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1053-1056)

```text
*******************************************************************************
  Line 1000                             &    Line 1100
    DIVS, DIVU, OR, SBCD                       ABCD, AND, EXG, MULS, MULU
*******************************************************************************
```

## EXG (lines 1204-1212)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        EXG       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
  .L :            |                 |
    Dx,Dy         |  6(1/0)         |                               np       n
    Ax,Ay         |  6(1/0)         |                               np       n
    Dx,Ay         |  6(1/0)         |                               np       n
```

