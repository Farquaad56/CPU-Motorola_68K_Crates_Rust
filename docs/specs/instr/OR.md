# OR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1053-1056)

```text
*******************************************************************************
  Line 1000                             &    Line 1100
    DIVS, DIVU, OR, SBCD                       ABCD, AND, EXG, MULS, MULU
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

## EORI, ORI, ANDI, SUBI, ADDI (lines 166-195)

```text
-------------------------------------------------------------------------------
 EORI, ORI, ANDI, |    Exec Time    |               Data Bus Usage
    SUBI, ADDI    |  INSTR     EA   | 1st Operand |  2nd OP (ea)  |   INSTR
------------------+-----------------+-------------+---------------+------------
#<data>,<ea> :    |                 |             |               |
  .B or .W :      |                 |             |               |
    Dn            |  8(2/0)  0(0/0) |          np |               | np
    (An)          | 12(2/1)  4(1/0) |          np |            nr | np nw
    (An)+         | 12(2/1)  4(1/0) |          np |            nr | np nw
    -(An)         | 12(2/1)  6(1/0) |          np | n          nr | np nw
    (d16,An)      | 12(2/1)  8(2/0) |          np |      np    nr | np nw
    (d8,An,Xn)    | 12(2/1) 10(2/0) |          np | n    np    nr | np nw
    (xxx).W       | 12(2/1)  8(2/0) |          np |      np    nr | np nw
    (xxx).L       | 12(2/1) 12(3/0) |          np |   np np    nr | np nw
  .L :                              |             |               |
    Dn            | 16(3/0)  0(0/0) |       np np |               | np       nn
    (An)          | 20(3/2)  8(2/0) |       np np |         nR nr | np nw nW
    (An)+         | 20(3/2)  8(2/0) |       np np |         nR nr | np nw nW
    -(An)         | 20(3/2) 10(2/0) |       np np | n       nR nr | np nw nW
    (d16,An)      | 20(3/2) 12(3/0) |       np np |      np nR nr | np nw nW
    (d8,An,Xn)    | 20(3/2) 14(3/0) |       np np | n    np nR nr | np nw nW
    (xxx).W       | 20(3/2) 12(3/0) |       np np |      np nR nr | np nw nW
    (xxx).L       | 20(3/2) 16(4/0) |       np np |   np np nR nr | np nw nW
NOTES :
  .M68000UM is probably wrong with instruction timming for ANDI.L #<data>,Dn
   instruction. It reads "14(3/0)" but, according to USP4325121, this
   instruction is based on the same microwords than EORI.L #<data>,Dn and
   ORI.L #<data>,Dn that have a "16(3/0)" timming. In addition, evaluation on
   real hardware confirms the 16 cycles timing.
```

## AND, OR (lines 1066-1108)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
      AND, OR     |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,Dn :         |                 |               |
  .B or .W :      |                 |               |
    Dn            |  4(1/0)  0(0/0) |               |               np
    (An)          |  4(1/0)  4(1/0) |            nr |               np
    (An)+         |  4(1/0)  4(1/0) |            nr |               np
    -(An)         |  4(1/0)  6(1/0) | n          nr |               np
    (d16,An)      |  4(1/0)  8(2/0) |      np    nr |               np
    (d8,An,Xn)    |  4(1/0) 10(2/0) | n    np    nr |               np
    (xxx).W       |  4(1/0)  8(2/0) |      np    nr |               np
    (xxx).L       |  4(1/0) 12(3/0) |   np np    nr |               np
    #<data>       |  4(1/0)  4(1/0) |      np       |               np
  .L :            |                 |               |
    Dn            |  8(1/0)  0(0/0) |               |               np       nn
    (An)          |  6(1/0)  8(2/0) |         nR nr |               np       n
    (An)+         |  6(1/0)  8(2/0) |         nR nr |               np       n
    -(An)         |  6(1/0) 10(2/0) | n       nR nr |               np       n
    (d16,An)      |  6(1/0) 12(3/0) |      np nR nr |               np       n
    (d8,An,Xn)    |  6(1/0) 14(3/0) | n    np nR nr |               np       n
    (xxx).W       |  6(1/0) 12(3/0) |      np nR nr |               np       n
    (xxx).L       |  6(1/0) 16(4/0) |   np np nR nr |               np       n
    #<data>       |  8(1/0)  8(2/0) |   np np       |               np       nn
Dn,<ea> :         |                 |               |
  .B or .W :      |                 |               |
    (An)          |  8(1/1)  4(1/0) |            nr |               np nw
    (An)+         |  8(1/1)  4(1/0) |            nr |               np nw
    -(An)         |  8(1/1)  6(1/0) | n          nr |               np nw
    (d16,An)      |  8(1/1)  8(2/0) |      np    nr |               np nw
    (d8,An,Xn)    |  8(1/1) 10(2/0) | n    np    nr |               np nw
    (xxx).W       |  8(1/1)  8(2/0) |      np    nr |               np nw
    (xxx).L       |  8(1/1) 12(3/0) |   np np    nr |               np nw
  .L :            |                 |
    (An)          | 12(1/2)  8(2/0) |         nR nr |               np nw nW
    (An)+         | 12(1/2)  8(2/0) |         nR nr |               np nw nW
    -(An)         | 12(1/2) 10(2/0) | n       nR nr |               np nw nW
    (d16,An)      | 12(1/2) 12(3/0) |      np nR nr |               np nw nW
    (d8,An,Xn)    | 12(1/2) 14(3/0) | n    np nR nr |               np nw nW
    (xxx).W       | 12(1/2) 12(3/0) |      np nR nr |               np nw nW
    (xxx).L       | 12(1/2) 16(4/0) |   np np nR nr |               np nw nW
```

