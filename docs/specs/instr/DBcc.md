# DBcc

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## DBcc (lines 954-977)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       DBcc       |      INSTR      |                   INSTR
------------------+-----------------+------------------------------------------
Dn,<label> :      |                 |
  branch taken    | 10(2/0)         |                      n np       np
  branch not taken|                 |
    cc true       | 12(2/0)         |                      n     n np np
    counter exp   | 14(3/0)         |                      n np    np np
NOTES :
  .DBcc does not exist in USP4325121. Only does a primitive instruction DCNT
   tht doesn't succeded to reach the production version of M68000 CPU.
   Nervertheless, looking at DCNT and Bcc micro/nanowords can lead to a
   realistic idea of how DBcc should work. At least, this re-enginering of DBcc
   matches the timing written in M68000UM for this instruction.
FLOWCHART :
                   cc                  n                 /cc
                   +-------------------+-------------------+
                   n                             z         np       /z
                   |                             +---------+---------+
                   +---------------------------->np                  |
                                                 |                   |
                                                 np<-----------------+
```

