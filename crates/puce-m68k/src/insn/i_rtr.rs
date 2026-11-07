//! RTR (`0x4E77`): pops CCR then PC. Timing: docs/specs/instr/RTR.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of RTR.
pub const ID: InsnId = InsnId(56);

const HK_EA: u8 = 0x20; // ea = SP
const HK_CCR: u8 = 0x21; // aux = CCR word, ea = SP + 2
const HK_JUMP: u8 = 0x22; // CCR, PC, SP += 6

/// Decoding rules of RTR.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFFF,
    value: 0x4E77,
    id: ID,
    variant: 0,
}];

static PLAN: [Tok; 7] = [
    Tok::H(HK_EA),
    Tok::Rw(AddrSrc::Ea),
    Tok::H(HK_CCR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];

/// Plan of RTR: CCR word, PC long, then two prefetches.
pub fn plan(_variant: u8, _cpu: &Cpu) -> Plan {
    &PLAN
}

/// Semantics of the hooks of RTR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_EA => cpu.lat.ea = cpu.regs.an(7),
        HK_CCR => {
            cpu.lat.aux = cpu.lat.val;
            cpu.lat.ea = cpu.regs.an(7).wrapping_add(2);
        }
        HK_JUMP => {
            cpu.regs.sr = (cpu.regs.sr & 0xFF00) | (cpu.lat.aux as u16 & 0x1F);
            cpu.regs.pc = cpu.lat.val;
            let sp = cpu.regs.an(7).wrapping_add(6);
            cpu.regs.set_an(7, sp);
        }
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

    #[test]
    fn e09_08_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 1);
        assert_eq!(decode(0x4E77).id, ID);
    }

    #[test]
    fn e09_08_decode_illegal() {
        for op in [0x4E76u16, 0x4E75, 0x4E73, 0x4E78] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_08_plan_basic() {
        assert_eq!(plan_clocks(plan(0, &Cpu::new())), 20);
    }

    #[test]
    fn e09_08_flags_sample() {
        // CCR word 0xFF15: only the low 5 bits count; the system byte of SR is kept
        let mut r = ram(&[(0x400, 0xFF15), (0x402, 0x0001), (0x404, 0x2000)]);
        let (cpu, n) = run(0x4E77, 0, 0, 0, 0x0003, &mut r);
        assert_eq!(
            (n, cpu.regs.pc, cpu.regs.sr, cpu.regs.usp),
            (20, 0x1_2004, 0x15, 0x406)
        );
    }
}
