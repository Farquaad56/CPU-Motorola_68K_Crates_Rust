# DIVS

Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').

## section banner (lines 1053-1056)

```text
*******************************************************************************
  Line 1000                             &    Line 1100
    DIVS, DIVU, OR, SBCD                       ABCD, AND, EXG, MULS, MULU
*******************************************************************************
```

## DIVS (lines 1153-1203)

```text
-------------------------------------------------------------------------------
                  |    Exec Time    |               Data Bus Usage
       DIVS       |  INSTR     EA   |  1st OP (ea)  |          INSTR
------------------+-----------------+---------------+--------------------------
<ea>,Dn :       /                  /            /
 .W :         /                  /          /
  Dn        |120+?(1/0)  0(0/0)|          | nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (An)      |120+?(1/0)  4(1/0)|        nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (An)+     |120+?(1/0)  4(1/0)|        nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  -(An)     |120+?(1/0)  6(1/0)|n       nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (d16,An)  |120+?(1/0)  8(2/0)|     np nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (d8,An,Xn)|120+?(1/0) 10(2/0)|n    np nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (xxx).W   |120+?(1/0)  8(2/0)|     np nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  (xxx).L   |120+?(1/0) 12(2/0)|  np np nr| nn nnn (n)n [seq] nnn-nnn n(n(n))np
  #<data>   |120+?(1/0)  8(2/0)|  np np   | nn nnn (n)n [seq] nnn-nnn n(n(n))np
NOTES :
  .Overflow cost 16 or 18 cycles depending on dividend sign (n nn nn (n)n np).
  .For more informations about division by zero see exceptions section below.
  .[seq.] refers to 15 consecutive blocks to be chosen in the following 2 :
   (nnn-n) or (nn-n).
   (see following flowchart for details).
  .Best case : 120-122 cycles depending on dividend sign.
  .Worst case : 156 (158 ?) cycles.
FLOWCHART :
                                         n
                                         n                       Divide by zero
                                +--------+--------------------------------+
                                n                                         n
                                n                                         n
                  Dividend<0    n    Dividend>0                           nw
                          +-----+-----+                                   nw
                          n           |                                   nw
                          +---------->n                                   np
                                      |    Overflow                       np
        +-----------------+      +----+----+                              np
        | +----------+    +----->n         np                             np
        | |          +---------->n
        | |                      n-
        | | MSB=1      MSB=0     n   No more bits
        | | +-----------+--------+--------+
        | +-+           |                 n
        +---------------+   divisor<0     n        divisor>0
                               +----------+-----------+
                               n         dividend<0   n   dividend>0
                               n           +----------+----+
                               np          n               np
                                           n
                                           np
  .for each iteration of the loop : shift quotient to the left by 1 bit.
  .MSB = most significant bit : bit at the far left of the quotient.
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

