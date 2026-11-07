# ROR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1448-1451)

```text
*******************************************************************************
  Line 1110
    ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR
*******************************************************************************
```

## ASL, ASR, LSL, LSR, ROL, ROR, ROXL, ROXR (lines 1452-1479)

```text
-------------------------------------------------------------------------------
     ASL, ASR,    |    Exec Time    |               Data Bus Usage
     LSL, LSR,    |                 |
     ROL, ROR,    |                 |
    ROXL, ROXR    |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
Dx,Dy :           |                 |               |
  .B or .W :      |  6+2m(1/0)      |               |               np    n* n
  .L :            |  8+2m(1/0)      |               |               np    n* nn
#<data>,Dy :      |                 |               |
  .B or .W :      |  6+2m(1/0)      |               |               np    n  n*
  .L :            |  8+2m(1/0)      |               |               np    nn n*
<ea> :            |                 |               |
  .B or .W :      |                 |               |
    (An)          |  8(1/1)  4(1/0) |            nr |               np    nw
    (An)+         |  8(1/1)  4(1/0) |            nr |               np    nw
    -(An)         |  8(1/1)  6(1/0) | n          nr |               np    nw
    (d16,An)      |  8(1/1)  8(2/0) |      np    nr |               np    nw
    (d8,An)       |  8(1/1) 10(2/0) | n    np    nr |               np    nw
    (xxx).W       |  8(1/1)  8(2/0) |      np    nr |               np    nw
    (xxx).L       |  8(1/1) 12(3/0) |   np np    nr |               np    nw
NOTES :
  .'m' is the shift count.
  .'n*' should be replaced by m consecutive 'n'
```

