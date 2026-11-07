# TRAP

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 560-564)

```text
*******************************************************************************
  Line 0100
    CHK, CLR, EXT, ILLEGAL, JMP, JSR, LEA , LINK, MV fr/to CCR/SR, MOVEM, NBCD,
    NEG, NEGX, NOP, NOT, PEA, RTR, RTS, SWAP, STOP, TAS, TRAP, TRAPV, TST, UNLK
*******************************************************************************
```

## ILLEGAL, TRAP (lines 905-915)

```text
-------------------------------------------------------------------------------
	                |    Exec Time    |               Data Bus Usage
  ILLEGAL, TRAP   |      INSTR      |                  INSTR
------------------+-----------------+------------------------------------------
                  | 34(4/3)         |          nn ns nS ns np np np np
NOTES :
  .For more informations about this instruction see exceptions section below.
```

## exception table (no title) (lines 1491-1533)

```text
-------------------------------------------------------------------------------
                      | Periods  |               Data Bus Usage
----------------------+----------+---------------------------------------------
                      |          |
Group 0 :             |          |
  Address error       | 50(4/7)  |48   nn ns ns nS ns ns ns nS nV nv np np
  Bus error           | 50(4/7)  |48   nn ns ns nS ns ns ns nS nV nv np np
  /RESET              | 40(6/0)  |38   (n-)*5   nn       nF nf nV nv np np
Group 1 :             |          |
  Trace               | 34(4/3)  |32            nn    ns nS ns nV nv np np
  Interrupt           | 44(5/3)  |42    n nn ns ni n-  n nS ns nV nv np np
  Privilege Violation | 34(4/3)  |32            nn    ns ns nS nV nv np np
  Illegal Instruction | 34(4/3)  |32            nn    ns ns nS nV nv np np
Group 2 :             |          |
  CHK Instruction     | 40(4/3)+ |38 np (n-)    nn    ns ns nS nV nv np np
  Divide by Zero      | 38(4/3)+ |36         nn nn    ns ns nS nV nv np np
  TRAP instruction    | 34(4/3)  |32            nn    ns ns nS nV nv np np
  TRAPV instruction   | 34(5/3)  |32 np               ns nS ns nV nv np np
NOTES:
  .for CHK, Divide by Zero, Illegal, Trap and TRAPV :
    .The numbers of clock periods, for the "trap" lines, include the times for
     all stacking, the vector fetch and the fetch of the first instruction of
     THE handler routine.
    .First are pushed on the stack the program counter high and low words then
     the status register.
  .for CHK and Divide by zero, you must add effective address calculation time
   to the period (according to instruction section above).
  ./RESET line indicates the time from when /RESET and /HALT are first sampled
   as negated to when instruction execution starts. For a subsequent external
   reset, asserting these signals for 10 clock cycles or longer is mandatory to
   initiate reset sequence. During these 10 clock cycles the CPU is in halted
   state (see below). However, an external reset signal that is asserted while
   the CPU is executing a reset instruction is ignored. Since the processor
   asserts the /RESET signal for 124 clock cycles during execution of a reset
   instruction, an external reset should assert /RESET for at least 132 clock
   periods.
  .For all these exceptions, there is a difference of 2 cycles between Data
   bus usage as obtained from USP4325121 and periods as written in M68000UM.
   There's no proven theory to explain this gap.
```

