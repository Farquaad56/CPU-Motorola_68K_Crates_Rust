//! Scc (`0101 cccc 11 ea`): sets the byte operand to 0xFF when the condition holds, else 0.
//! No flags. Timing: docs/specs/instr/Scc.md (byte read-modify-write; `np n` on Dn when true).

use crate::alu::{scc_hook, scc_plan, scc_rules, N_RULES_SCC};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of Scc.
pub const ID: InsnId = InsnId(52);

static RULES_ARR: [Rule; N_RULES_SCC] = scc_rules(ID);

/// Decoding rules of Scc.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operand and the condition).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    scc_plan(cpu)
}

/// Semantics of the hooks of Scc.
pub fn hook(k: u8, cpu: &mut Cpu) {
    scc_hook(k, cpu);
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

    fn clocks(op: u16, sr: u16) -> u32 {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        cpu.regs.sr = sr;
        plan_clocks(plan(0, &cpu))
    }

    #[test]
    fn e09_04_decode_legal() {
        assert_eq!(
            TABLE.iter().filter(|d| d.id == ID).count(),
            16 * (8 + 40 + 2)
        );
    }

    #[test]
    fn e09_04_decode_illegal() {
        for op in [0x50C8u16, 0x50FA, 0x50FC, 0x5040, 0x5080] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_04_plan_basic() {
        assert_eq!(clocks(0x50C0, 0), 6); // ST D0
        assert_eq!(clocks(0x51C0, 0), 4); // SF D0
        assert_eq!(clocks(0x50D0, 0), 12); // ST (A0)
        assert_eq!(clocks(0x50F9, 0), 20); // ST (xxx).L
    }

    #[test]
    fn e09_04_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x50C0, 0, 0, 0x1234_5600, 0x15, &mut r); // ST D0
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr), (6, 0x1234_56FF, 0x15));
        let (cpu, n) = run(0x51C0, 0, 0, 0xFFFF_FFFF, 0, &mut r); // SF D0
        assert_eq!((n, cpu.regs.d[0]), (4, 0xFFFF_FF00));
        let (cpu, _) = run(0x57C0, 0, 0, 0, 4, &mut r); // SEQ, Z set
        assert_eq!(cpu.regs.d[0], 0xFF);
        let (cpu, _) = run(0x57C0, 0, 0, 0xFF, 0, &mut r); // SEQ, Z clear
        assert_eq!(cpu.regs.d[0], 0);
    }
}
