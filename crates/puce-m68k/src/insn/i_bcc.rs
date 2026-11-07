//! Bcc (`0110 cccc dddddddd`, cond 0 = BRA, 2..15; cond 1 is BSR) with an 8-bit displacement or,
//! when it is 0, a word displacement in the extension word. Base = address of the opcode + 2.
//! Flags untouched. Timing: docs/specs/instr/Bcc.md (taken `n np np`; not taken `nn np`, `.W`
//! `nn np np`). The condition only depends on SR, so the plan is chosen when the instruction starts.

use crate::alu::cond_true;
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};

/// Identifier of Bcc.
pub const ID: InsnId = InsnId(49);

/// Hook: loads PC with the branch target (before the `Pf` that consumes the extension word).
pub const HK_BR: u8 = 0x91;

const fn build_rules() -> [Rule; 15] {
    let mut out = [Rule {
        mask: 0,
        value: 0,
        id: InsnId::ILLEGAL,
        variant: 0,
    }; 15];
    let mut k = 0;
    let mut c = 0u16;
    while c < 16 {
        if c != 1 {
            out[k] = Rule {
                mask: 0xFF00,
                value: 0x6000 | (c << 8),
                id: ID,
                variant: 0,
            };
            k += 1;
        }
        c += 1;
    }
    out
}

static RULES_ARR: [Rule; 15] = build_rules();

/// Decoding rules of Bcc.
pub static RULES: &[Rule] = &RULES_ARR;

static TAKEN: [Tok; 4] = [Tok::N(2), Tok::H(HK_BR), Tok::Pf, Tok::Pf];
static NOT_TAKEN_B: [Tok; 2] = [Tok::N(4), Tok::Pf];
static NOT_TAKEN_W: [Tok; 3] = [Tok::N(4), Tok::Pf, Tok::Pf];

/// Plan of the instruction being executed: depends on the condition and the displacement size.
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    if cond_true(ird >> 8, cpu.regs.sr) {
        &TAKEN
    } else if ird & 0xFF == 0 {
        &NOT_TAKEN_W
    } else {
        &NOT_TAKEN_B
    }
}

/// Branch target: `opcode address + 2 + displacement`.
pub fn target(ird: u16, irc: u16, pc: u32) -> u32 {
    let disp = if ird & 0xFF == 0 {
        i32::from(irc as i16)
    } else {
        i32::from(ird as u8 as i8)
    };
    pc.wrapping_sub(2).wrapping_add(disp as u32)
}

/// Semantics of the hooks of Bcc.
pub fn hook(k: u8, cpu: &mut Cpu) {
    if k == HK_BR {
        cpu.regs.pc = target(cpu.lat.ird, cpu.regs.irc, cpu.regs.pc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::state::CpuState;

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, ext: u16, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            sr,
            pc: 0x20,
            prefetch: [op, ext],
            ..CpuState::default()
        });
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    fn clocks(op: u16, sr: u16) -> u32 {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        cpu.regs.sr = sr;
        plan_clocks(plan(0, &cpu))
    }

    #[test]
    fn e09_01_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 15 * 256);
        assert_eq!(decode(0x6000).id, ID);
        assert_eq!(decode(0x6FFF).id, ID);
    }

    #[test]
    fn e09_01_decode_illegal() {
        // BSR, DBcc, Scc and the neighbouring lines
        for op in [0x6100, 0x61FF, 0x50C8, 0x50C0, 0x5FFF, 0x7000] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_01_plan_basic() {
        assert_eq!(clocks(0x6001, 0), 10); // BRA.B
        assert_eq!(clocks(0x6000, 0), 10); // BRA.W
        assert_eq!(clocks(0x6601, 0x04), 8); // BNE.B, Z set: not taken
        assert_eq!(clocks(0x6600, 0x04), 12); // BNE.W not taken
        assert_eq!(clocks(0x6701, 0), 8); // BEQ.B, Z clear: not taken
    }

    #[test]
    fn e09_01_flags_sample() {
        // BEQ.B +4 taken: target = 0x1C + 2 + 4 = 0x22, then two prefetches
        let (cpu, n) = run(0x6704, 0, 0x04);
        assert_eq!((n, cpu.regs.pc, cpu.regs.sr), (10, 0x26, 0x04));
        // same instruction, Z clear: falls through
        let (cpu, n) = run(0x6704, 0, 0x1B);
        assert_eq!((n, cpu.regs.pc, cpu.regs.sr), (8, 0x22, 0x1B));
        // BRA.W -6: base 0x1E - 6 = 0x18, two prefetches
        let (cpu, n) = run(0x6000, 0xFFFA, 0);
        assert_eq!((n, cpu.regs.pc), (10, 0x1C));
    }
}
