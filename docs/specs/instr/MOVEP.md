# MOVEP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 160-164)

```text
*******************************************************************************
  Line 0000 :
    ADDI, ANDI, ANDI to CCR, BCHG, BCLR, BSET, BTST, CMPI, EORI, EORI to CCR,
    MOVEP, ORI, ORI to CCR, SUBI
*******************************************************************************
```

## MOVEP (lines 316-334)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
      MOVEP       |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
Dx,(d16,Ay) :     |                 |
  .W :            | 16(2/2)         |                np    nW    nw np
  .L :            | 24(2/4)         |                np nW nW nw nw np
(d16,Ay),Dx :     |                 |
  .W :            | 16(4/0)         |                np    nR    nr np
  .L :            | 24(6/0)         |                np nR nR nr nr np
NOTES :
  .Read and write operations are done from the MSB to the LSB on 2 words if
  using ".w" (first read/write word at Ay+d16 then word at Ay+d16+2)and on
  4 words if using ".l" (first read/write word at Ay+d16 then word at Ay+d16+2
  then word at Ay+d16+4 and finally word at Ay+d16+6).
```

