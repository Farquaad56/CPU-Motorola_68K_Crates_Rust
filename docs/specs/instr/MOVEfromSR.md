# MOVEfromSR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## MOVE from SR (lines 590-604)

```text
-------------------------------------------------------------------------------
       MOVE       |    Exec Time    |               Data Bus Usage
     from SR      |  INSTR     EA   |  2nd Op (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
SR,<ea> :         |                 |               |
  .W :            |                 |               |
    Dn            |  6(1/0)  0(0/0) |               |               np       n
    (An)          |  8(1/1)  4(1/0) |            nr |               np nw
    (An)+         |  8(1/1)  4(1/0) |            nr |               np nw
    -(An)         |  8(1/1)  6(1/0) | n          nr |               np nw
    (d16,An)      |  8(1/1)  8(2/0) |      np    nr |               np nw
    (d8,An,Xn)    |  8(1/1) 10(2/0) | n    np    nr |               np nw
    (xxx).W       |  8(1/1)  8(2/0) |      np    nr |               np nw
    (xxx).L       |  8(1/1) 12(3/0) |   np np    nr |               np nw
```

