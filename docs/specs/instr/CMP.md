# CMP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1353-1356)

```text
*******************************************************************************
  Line 1011
    CMP, CMPA, CMPM, EOR
*******************************************************************************
```

## section banner (lines 160-164)

```text
*******************************************************************************
  Line 0000 :
    ADDI, ANDI, ANDI to CCR, BCHG, BCLR, BSET, BTST, CMPI, EORI, EORI to CCR,
    MOVEP, ORI, ORI to CCR, SUBI
*******************************************************************************
```

## CMPI (lines 203-226)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       CMPI       |  INSTR     EA   | 1st Operand |  2nd OP (ea)  |   INSTR
------------------+-----------------+-------------+-------------+--------------
#<data>,<ea> :    |                 |             |               |
  .B or .W :      |                 |             |               |
    Dn            |  8(2/0)  0(0/0) |          np |               | np
    (An)          |  8(2/0)  4(1/0) |          np |            nr | np
    (An)+         |  8(2/0)  4(1/0) |          np |            nr | np
    -(An)         |  8(2/0)  6(1/0) |          np | n          nr | np
    (d16,An)      |  8(2/0)  8(2/0) |          np |      np    nr | np
    (d8,An,Xn)    |  8(2/0) 10(2/0) |          np | n    np    nr | np
    (xxx).W       |  8(2/0)  8(2/0) |          np |      np    nr | np
    (xxx).L       |  8(2/0) 12(3/0) |          np |   np np    nr | np
  .L :            |                 |             |               |
    Dn            | 14(3/0)  0(0/0) |       np np |               | np       n
    (An)          | 12(3/0)  8(2/0) |       np np |         nR nr | np
    (An)+         | 12(3/0)  8(2/0) |       np np |         nR nr | np
    -(An)         | 12(3/0) 10(2/0) |       np np | n       nR nr | np
    (d16,An)      | 12(3/0) 12(3/0) |       np np |      np nR nr | np
    (d8,An,Xn)    | 12(3/0) 14(3/0) |       np np | n    np nR nr | np
    (xxx).W       | 12(3/0) 12(3/0) |       np np |      np nR nr | np
    (xxx).L       | 12(3/0) 16(4/0) |       np np |   np np nR nr | np
```

## CMP (lines 1357-1384)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       CMP        |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,Dn :         |                 |               |
  .B or .W :      |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    An            |  4(1/0)  0(0/0) |               |               np
    (An)          |  4(1/0)  4(1/0) |            nr |               np
    (An)+         |  4(1/0)  4(1/0) |            nr |               np
    -(An)         |  4(1/0)  6(1/0) | n          nr |               np
    (d16,An)      |  4(1/0)  8(2/0) |      np    nr |               np
    (d8,An,Xn)    |  4(1/0) 10(2/0) | n    np    nr |               np
    (xxx).W       |  4(1/0)  8(2/0) |      np    nr |               np
    (xxx).L       |  4(1/0) 12(3/0) |   np np    nr |               np
    #<data>       |  4(1/0)  4(1/0) |      np       |               np
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

## CMPM (lines 1413-1420)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       CMPM       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
(Ay)+,(Ax)+       |                 |
  .B or .W :      | 12(3/0)         |                      nr    nr np
  .L :            | 20(5/0)         |                   nR nr nR nr np
```

