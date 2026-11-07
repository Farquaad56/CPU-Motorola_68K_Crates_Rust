# MOVEtoSR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## MOVE to CCR, to SR (lines 605-620)

```text
-------------------------------------------------------------------------------
       MOVE       |    Exec Time    |               Data Bus Usage
  to CCR, to SR   |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,CCR :        |                 |               |
  .W :            |                 |               |
    Dn            | 12(1/0)  0(0/0) |               |         nn np np
    (An)          | 12(1/0)  4(1/0) |            nr |         nn np np
    (An)+         | 12(1/0)  4(1/0) |            nr |         nn np np
    -(An)         | 12(1/0)  6(1/0) | n          nr |         nn np np
    (d16,An)      | 12(1/0)  8(2/0) |      np    nr |         nn np np
    (d8,An,Xn)    | 12(1/0) 10(2/0) | n    np    nr |         nn np np
    (xxx).W       | 12(1/0)  8(2/0) |      np    nr |         nn np np
    (xxx).L       | 12(1/0) 12(3/0) |   np np    nr |         nn np np
    #<data>       | 12(1/0)  4(1/0) |      np       |         nn np np
```

