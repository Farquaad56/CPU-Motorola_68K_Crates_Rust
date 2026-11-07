# Bcc

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## BRA (lines 1010-1016)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        BRA       |      INSTR      |                   INSTR
------------------+-----------------+------------------------------------------
<label> :         |                 |
  .B, .S or .W :  | 10(2/0)         |                 n          np np
```

## Bcc (lines 1024-1038)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
        Bcc       |      INSTR      |                   INSTR
------------------+-----------------+------------------------------------------
<label> :         |                 |
 .B or .S :       |                 |
  branch taken    | 10(2/0)         |                 n          np np
  branch not taken|  8(1/0)         |                nn             np
 .W :             |                 |
  branch taken    | 10(2/0)         |                 n          np np
  branch not taken| 12(2/0)         |                nn          np np
```

