# MULU

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1053-1056)

```text
*******************************************************************************
  Line 1000                             &    Line 1100
    DIVS, DIVU, OR, SBCD                       ABCD, AND, EXG, MULS, MULU
*******************************************************************************
```

## MULU,MULS (lines 1213-1253)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
    MULU,MULS     |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,Dn :        /                  |               |
  .W :          /                   |               |
    Dn         | 38+2m(1/0)  0(0/0) |               |               np       n*
    (An)       | 38+2m(1/0)  4(1/0) |            nr |               np       n*
    (An)+      | 38+2m(1/0)  4(1/0) |            nr |               np       n*
    -(An)      | 38+2m(1/0)  6(1/0) | n          nr |               np       n*
    (d16,An)   | 38+2m(1/0)  8(2/0) |      np    nr |               np       n*
    (d8,An,Xn) | 38+2m(1/0) 10(2/0) | n    np    nr |               np       n*
    (xxx).W    | 38+2m(1/0)  8(2/0) |      np    nr |               np       n*
    (xxx).L    | 38+2m(1/0) 12(2/0) |   np np    nr |               np       n*
    #<data>    | 38+2m(1/0)  8(2/0) |   np np       |               np       n*
NOTES :
  .for MULU 'm' = the number of ones in the source
    - Best case 38 cycles with $0
    - Worst case : 70 cycles with $FFFF
  .for MULS 'm' = concatenate the 16-bit pointed by <ea> with a zero as the LSB
   'm' is the resultant number of 10 or 01 patterns in the 17-bit source.
    - Best case : 38 cycles with $0 or $FFFF
    - Worst case : 70 cycles with $5555
  .in both cases : 'n*' should be replaced by 17+m consecutive 'n'
FLOWCHART :
                            np
         LSB=1              |           LSB=0
               +------------+------------+
               |                         |
            +->n------------------------>n<----+
            |                            |     | LSB=0 or
            |           No more bits     |     | 2LSB=00,11
   LSB=1 or +---------------+------------+-----+
   2LSB=01,10               |
                            n
  .LSB = less significant bit : bit at the far right of the source.
  .2LSB = 2 less significant bits : 2 last bits at the far right of the source.
```

