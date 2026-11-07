//! MOVE from SR (`0x40C0 | ea`): writes SR (word) to a data-alterable operand. Not privileged on the
//! 68000. Timing: docs/specs/instr/MOVEfromSR.md.

use crate::alu::{mode_rules, move_from_sr_hook, move_from_sr_plan};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of MOVEfromSR.
pub const ID: InsnId = InsnId(65);

static RULES_ARR: [Rule; 8] = mode_rules::<8>(0x40C0, ID);

/// Decoding rules of MOVEfromSR.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of MOVEfromSR: `np n` on Dn, `nr np nw` on memory.
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    move_from_sr_plan(cpu)
}

/// Semantics of the hooks of MOVEfromSR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    move_from_sr_hook(k, cpu);
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

    /// Runs one instruction at PC = 0x20 (opcode at 0x1C): USP 0x400, SSP 0x800.
    fn run(op: u16, ext: u16, a0: u32, d0: u32, sr: u16, r: &mut Ram) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [a0, 0, 0, 0, 0, 0, 0],
            d: [d0, 0, 0, 0, 0, 0, 0, 0],
            usp: 0x400,
            ssp: 0x800,
            sr,
            pc: 0x20,
            prefetch: [op, ext],
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

    /// Vector 8 (privilege violation) points to 0x1000.
    #[allow(dead_code)]
    fn vec_ram() -> Ram {
        ram(&[(0x20, 0x0000), (0x22, 0x1000)])
    }

    #[test]
    fn e10_09_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 + 40 + 2);
    }

    #[test]
    fn e10_09_decode_illegal() {
        for op in [0x40C8u16, 0x40FA, 0x40FC, 0x4040] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e10_09_plan_basic() {
        assert_eq!(clocks(0x40C0, 0x2700), 6); // Dn
        assert_eq!(clocks(0x40D0, 0x2700), 12); // (A0)
        assert_eq!(clocks(0x40F9, 0x2700), 20); // (xxx).L
    }

    #[test]
    fn e10_09_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x40C0, 0, 0, 0xFFFF_FFFF, 0x2715, &mut r);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr), (6, 0xFFFF_2715, 0x2715));
        // not privileged on the 68000
        let (cpu, _) = run(0x40C0, 0, 0, 0, 0x0015, &mut r);
        assert_eq!(cpu.regs.d[0], 0x0015);
        let (_, n) = run(0x40D0, 0, 0x1000, 0, 0x0015, &mut r);
        assert_eq!((n, r.writes.clone()), (12, std::vec![(0x1000, 0x0015)]));
    }
}
