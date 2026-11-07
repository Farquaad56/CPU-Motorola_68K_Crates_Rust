//! MOVEfromUSP (`0x4e68 | An`): An = USP. Privileged. Timing: docs/specs/instr/MOVEfromUSP.md.

use crate::alu::{usp_hook, usp_plan};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of MOVEfromUSP.
pub const ID: InsnId = InsnId(67);

/// Decoding rules of MOVEfromUSP.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFF8,
    value: 0x4e68,
    id: ID,
    variant: 0,
}];

/// Plan of MOVEfromUSP: `np` (privileged).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    usp_plan(cpu)
}

/// Semantics of the hooks of MOVEfromUSP.
pub fn hook(k: u8, cpu: &mut Cpu) {
    usp_hook(false, k, cpu);
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
    fn e10_11_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8);
    }

    #[test]
    fn e10_11_decode_illegal() {
        for op in [0x4e60u16, 0x4e78, 0x4E70, 0x4E58] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e10_11_plan_basic() {
        assert_eq!(clocks(0x4e68, 0x2700), 4);
    }

    #[test]
    fn e10_11_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4E68, 0, 0, 0, 0x2700, &mut r);
        assert_eq!((n, cpu.regs.a[0], cpu.regs.pc), (4, 0x400, 0x22));
        let mut r2 = vec_ram();
        let (cpu, n) = run(0x4E68, 0, 0x1234, 0, 0x0015, &mut r2);
        assert_eq!((n, cpu.regs.pc, cpu.regs.a[0]), (34, 0x1004, 0x1234));
    }
}
