//! DBcc (`0101 cccc 1100 1 rrr` + word displacement): if the condition is false, decrements Dn.w and
//! branches unless it became -1. Base = address of the extension word. No flags.
//! Timing: docs/specs/instr/DBcc.md. The expired-counter prefetch order (target first) follows its
//! flowchart; the SST files hold no such case.

use crate::alu::cond_true;
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};

/// Identifier of DBcc.
pub const ID: InsnId = InsnId(51);

const HK_DB: u8 = 0x20; // Dn.w -= 1, PC = target
const HK_DB_EXP: u8 = 0x21; // Dn.w -= 1, aux2 = PC, PC = target
const HK_RESTORE: u8 = 0x22; // PC = aux2

/// Decoding rules of DBcc (`0101 cccc 1100 1 rrr`).
pub const RULES: &[Rule] = &[Rule {
    mask: 0xF0F8,
    value: 0x50C8,
    id: ID,
    variant: 0,
}];

static CC_TRUE: [Tok; 3] = [Tok::N(4), Tok::Pf, Tok::Pf];
static TAKEN: [Tok; 4] = [Tok::N(2), Tok::H(HK_DB), Tok::Pf, Tok::Pf];
static EXPIRED: [Tok; 6] = [
    Tok::N(2),
    Tok::H(HK_DB_EXP),
    Tok::Pf,
    Tok::H(HK_RESTORE),
    Tok::Pf,
    Tok::Pf,
];

/// Plan of DBcc: condition true `nn np np`, loop `n np np`, counter expired `n np np np`.
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    let ird = cpu.lat.ird;
    if cond_true(ird >> 8, cpu.regs.sr) {
        &CC_TRUE
    } else if cpu.regs.d[(ird & 7) as usize] & 0xFFFF != 0 {
        &TAKEN
    } else {
        &EXPIRED
    }
}

fn decrement(cpu: &mut Cpu) {
    let n = (cpu.lat.ird & 7) as usize;
    let d = cpu.regs.d[n];
    cpu.regs.d[n] = (d & 0xFFFF_0000) | (d.wrapping_sub(1) & 0xFFFF);
    cpu.regs.pc = cpu
        .regs
        .pc
        .wrapping_sub(2)
        .wrapping_add(i32::from(cpu.regs.irc as i16) as u32);
}

/// Semantics of the hooks of DBcc.
pub fn hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_DB => decrement(cpu),
        HK_DB_EXP => {
            cpu.lat.aux2 = cpu.regs.pc;
            decrement(cpu);
        }
        HK_RESTORE => cpu.regs.pc = cpu.lat.aux2,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::state::CpuState;

    struct Ram {
        writes: std::vec::Vec<(u32, u16)>,
        mem: std::vec::Vec<(u32, u16)>,
    }
    impl Bus for Ram {
        fn read(&mut self, a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            self.mem
                .iter()
                .find(|(x, _)| *x == a)
                .map_or(0, |(_, d)| *d)
        }
        fn write(&mut self, a: u32, _f: u8, _u: bool, _l: bool, d: u16) {
            self.writes.push((a, d));
        }
    }

    fn ram(mem: &[(u32, u16)]) -> Ram {
        Ram {
            writes: std::vec::Vec::new(),
            mem: mem.to_vec(),
        }
    }

    fn run(op: u16, ext: u16, a0: u32, d0: u32, sr: u16, r: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [a0, 0, 0, 0, 0, 0, 0],
            d: [d0, 0, 0, 0, 0, 0, 0, 0],
            usp: 0x400,
            sr,
            pc: 0x20,
            prefetch: [op, ext],
            ..CpuState::default()
        });
        let n = cpu.step(r);
        (cpu, n)
    }

    fn clocks(op: u16, sr: u16, d0: u32) -> u32 {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        cpu.regs.sr = sr;
        cpu.regs.d[0] = d0;
        plan_clocks(plan(0, &cpu))
    }

    #[test]
    fn e09_03_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 16 * 8);
    }

    #[test]
    fn e09_03_decode_illegal() {
        for op in [0x50C0u16, 0x50D0, 0x50C8 ^ 0x40, 0x5048, 0x60C8] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_03_plan_basic() {
        assert_eq!(clocks(0x51C8, 0, 5), 10); // DBF: loop
        assert_eq!(clocks(0x51C8, 0, 0x1_0000), 14); // DBF: counter expired
        assert_eq!(clocks(0x50C8, 0, 5), 12); // DBT: condition true
    }

    #[test]
    fn e09_03_flags_sample() {
        let mut r = ram(&[]);
        // DBF D0,+8 with D0.w = 2: loops to 0x1E + 8 = 0x26
        let (cpu, n) = run(0x51C8, 0x0008, 0, 0x1234_0002, 0x15, &mut r);
        assert_eq!(
            (n, cpu.regs.pc, cpu.regs.d[0], cpu.regs.sr),
            (10, 0x2A, 0x1234_0001, 0x15)
        );
        // DBT: condition true, falls through, D0 untouched
        let (cpu, n) = run(0x50C8, 0x0008, 0, 0x1234_0002, 0, &mut r);
        assert_eq!((n, cpu.regs.pc, cpu.regs.d[0]), (12, 0x24, 0x1234_0002));
        // DBF with D0.w = 0: becomes 0xFFFF, falls through
        let (cpu, n) = run(0x51C8, 0x0008, 0, 0x1234_0000, 0, &mut r);
        assert_eq!((n, cpu.regs.pc, cpu.regs.d[0]), (14, 0x24, 0x1234_FFFF));
    }
}
