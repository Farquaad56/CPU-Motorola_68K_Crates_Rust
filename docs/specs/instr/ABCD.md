# ABCD

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1053-1056)

```text
*******************************************************************************
  Line 1000                             &    Line 1100
    DIVS, DIVU, OR, SBCD                       ABCD, AND, EXG, MULS, MULU
*******************************************************************************
```

## ABCD, SBCD (lines 1057-1065)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
    ABCD, SBCD    |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
Dy,Dx :           |                 |
  .B :            |  6(1/0)         |                               np       n
-(Ay),-(Ax) :     |                 |
  .B :            | 18(3/1)         |                 n    nr    nr np nw
```

