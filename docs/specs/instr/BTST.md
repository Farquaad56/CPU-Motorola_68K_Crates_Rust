# BTST

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 160-164)

```text
*******************************************************************************
  Line 0000 :
    ADDI, ANDI, ANDI to CCR, BCHG, BCLR, BSET, BTST, CMPI, EORI, EORI to CCR,
    MOVEP, ORI, ORI to CCR, SUBI
*******************************************************************************
```

## BTST (lines 289-315)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       BTST       |  INSTR     EA   | 1st Operand |  2nd OP (ea)  |   INSTR
------------------+-----------------+-------------+---------------+------------
Dn,<ea> :         |                 |             |               |
  .B :            |                 |             |               |
    (An)          |  4(1/0)  4(1/0) |             |            nr | np
    (An)+         |  4(1/0)  4(1/0) |             |            nr | np
    -(An)         |  4(1/0)  6(1/0) |             | n          nr | np
    (d16,An)      |  4(1/0)  8(2/0) |             |      np    nr | np
    (d8,An,Xn)    |  4(1/0) 10(2/0) |             | n    np    nr | np
    (xxx).W       |  4(1/0)  8(2/0) |             |      np    nr | np
    (xxx).L       |  4(1/0) 12(3/0) |             |   np np    nr | np
Dn,Dm :           |                 |             |               |
  .L :            |  6(1/0)  0(0/0) |             |               | np n
#<data>,<ea> :    |                 |             |               |
  .B :            |                 |             |               |
    (An)          |  8(2/0)  4(1/0) |          np |            nr | np
    (An)+         |  8(2/0)  4(1/0) |          np |            nr | np
    -(An)         |  8(2/0)  6(1/0) |          np | n          nr | np
    (d16,An)      |  8(2/0)  8(2/0) |          np |      np    nr | np
    (d8,An,Xn)    |  8(2/0) 10(2/0) |          np | n    np    nr | np
    (xxx).W       |  8(2/0)  8(2/0) |          np |      np    nr | np
    (xxx).L       |  8(2/0) 12(3/0) |          np |   np np    nr | np
#<data>,Dn :      |                 |             |               |
  .L :            | 10(2/0)  0(0/0) |          np |               | np n
```

