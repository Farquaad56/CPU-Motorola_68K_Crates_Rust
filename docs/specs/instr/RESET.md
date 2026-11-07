# RESET

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## RESET (lines 840-850)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       RESET      |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
                  | 132(1/0)        |                      nn (??-)* np
NOTES :
  .(??)* is for 124 cycles when the CPU will asserts the /RSTO signal in order
   to reset all external devices. Normally, the CPU won't use the bus during
   these cycles.
  .RESET instruction have nothing to do with /RESET exception.
```

