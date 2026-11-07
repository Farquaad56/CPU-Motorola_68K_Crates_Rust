//! EORItoSR (`0x0a7c` + immediate word): SR = SR XOR immediate (privileged). Timing: docs/specs/instr/EORItoSR.md.

use crate::alu::{sri_hook, sri_plan, Alu};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of EORItoSR.
pub const ID: InsnId = InsnId(62);

/// Decoding rules of EORItoSR.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFFFF,
    value: 0x0a7c,
    id: ID,
    variant: 0,
}];

/// Plan of EORItoSR: `np`, `nn nn`, `np np` (the first `np` re-reads PC).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    sri_plan(cpu, true)
}

/// Semantics of the hooks of EORItoSR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    sri_hook(Alu::Eor, true, k, cpu);
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
    fn e10_06_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 1);
        assert_eq!(decode(0x0a7c).id, ID);
    }

    #[test]
    fn e10_06_decode_illegal() {
        for op in [0x0a3cu16, 0x0a7d, 0x0a74, 0x4E71] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e10_06_plan_basic() {
        assert_eq!(clocks(0x0a7c, 0x2700), 20);
    }

    #[test]
    fn e10_06_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x0A7C, 0x2000, 0, 0, 0x2015, &mut r);
        assert_eq!((n, cpu.regs.pc, cpu.regs.sr), (20, 0x24, 0x0015));
        let (cpu, _) = run(0x0A7C, 0x8000, 0, 0, 0x2700, &mut r);
        assert_eq!(cpu.regs.sr, 0xA700);
        // user mode: privilege violation (vector 8 -> 0x1000), frame at SSP 0x800
        let mut r = vec_ram();
        let (cpu, n) = run(0x0a7c, 0x2000, 0x1234, 0, 0x0015, &mut r);
        assert_eq!(
            (n, cpu.regs.pc, cpu.regs.sr, cpu.regs.ssp),
            (34, 0x1004, 0x2015, 0x7FA)
        );
        assert_eq!(
            r.writes,
            std::vec![(0x7FE, 0x1C), (0x7FA, 0x0015), (0x7FC, 0)]
        );
    }
}
