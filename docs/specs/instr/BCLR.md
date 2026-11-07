# BCLR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 160-164)

```text
*******************************************************************************
  Line 0000 :
    ADDI, ANDI, ANDI to CCR, BCHG, BCLR, BSET, BTST, CMPI, EORI, EORI to CCR,
    MOVEP, ORI, ORI to CCR, SUBI
*******************************************************************************
```

## BCLR (lines 258-288)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
     BCLR         |  INSTR     EA   | 1st Operand |  2nd OP (ea)  |   INSTR
------------------+-----------------+-------------+---------------+------------
Dn,<ea> :         |                 |             |               |
  .B :            |                 |             |               |
    (An)          |  8(1/1)  4(1/0) |             |            nr | np    nw
    (An)+         |  8(1/1)  4(1/0) |             |            nr | np    nw
    -(An)         |  8(1/1)  6(1/0) |             | n          nr | np    nw
    (d16,An)      |  8(1/1)  8(2/0) |             |      np    nr | np    nw
    (d8,An,Xn)    |  8(1/1) 10(2/0) |             | n    np    nr | np    nw
    (xxx).W       |  8(1/1)  8(2/0) |             |      np    nr | np    nw
    (xxx).L       |  8(1/1) 12(3/0) |             |   np np    nr | np    nw
Dn,Dm :           |                 |             |               |
  .L :            |                 |             |               |
    if Dn<16      |  8(1/0)  0(0/0) |             |               | np nn
    if Dn>15      | 10(1/0)  0(0/0) |             |               | np nn n
#<data>,<ea> :    |                 |             |               |
  .B :            |                 |             |               |
    (An)          | 12(2/1)  4(1/0) |          np |            nr | np    nw
    (An)+         | 12(2/1)  4(1/0) |          np |            nr | np    nw
    -(An)         | 12(2/1)  6(1/0) |          np | n          nr | np    nw
    (d16,An)      | 12(2/1)  8(2/0) |          np |      np    nr | np    nw
    (d8,An,Xn)    | 12(2/1) 10(2/0) |          np | n    np    nr | np    nw
    (xxx).W       | 12(2/1)  8(2/0) |          np |      np    nr | np    nw
    (xxx).L       | 12(2/1) 12(3/0) |          np |   np np    nr | np    nw
#<data>,Dn :      |                 |             |               |
  .L :            |                 |             |               |
    if data<16    | 12(2/0)  0(0/0) |          np |               | np nn
    if data>15    | 14(2/0)  0(0/0) |          np |               | np nn n
```

