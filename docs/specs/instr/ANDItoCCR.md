# ANDItoCCR

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## ORI, ANDI, EORI to CCR, to SR (lines 196-202)

```text
-------------------------------------------------------------------------------
 ORI, ANDI, EORI  |    Exec Time    |               Data Bus Usage
  to CCR, to SR   |      INSTR      | 1st Operand |          INSTR
------------------+-----------------+-------------+----------------------------
#<data>,CCR       |                 |             |
  .B :            | 20(3/0)         |          np |              nn nn np np
```

