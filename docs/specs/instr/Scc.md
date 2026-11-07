# Scc

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## Scc (lines 978-1005)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        Scc       |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea> :            |                 |               |
  .B :            |                 |               |
    cc false      |                 |               |
      Dn          |  4(1/0)  0(0/0) |               |               np
      (An)        |  8(1/1)  4(1/0) |            nr |               np    nw
      (An)+       |  8(1/1)  4(1/0) |            nr |               np    nw
      -(An)       |  8(1/1)  6(1/0) | n          nr |               np    nw
      (d16,An)    |  8(1/1)  8(2/0) |      np    nr |               np    nw
      (d8,An,Xn)  |  8(1/1) 10(2/0) | n    np    nr |               np    nw
      (xxx).W     |  8(1/1)  8(2/0) |      np    nr |               np    nw
      (xxx).L     |  8(1/1) 12(3/0) |   np np    nr |               np    nw
    cc true       |                 |               |
      Dn          |  6(1/0)  0(0/0) |               |               np       n
      (An)        |  8(1/1)  4(1/0) |            nr |               np    nw
      (An)+       |  8(1/1)  4(1/0) |            nr |               np    nw
      -(An)       |  8(1/1)  6(1/0) | n          nr |               np    nw
      (d16,An)    |  8(1/1)  8(2/0) |      np    nr |               np    nw
      (d8,An,Xn)  |  8(1/1) 10(2/0) | n    np    nr |               np    nw
      (xxx).W     |  8(1/1)  8(2/0) |      np    nr |               np    nw
      (xxx).L     |  8(1/1) 12(3/0) |   np np    nr |               np    nw
```

