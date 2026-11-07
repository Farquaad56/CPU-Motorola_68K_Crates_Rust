//! MOVEtoCCR (`0x44c0 | ea`): word source (any data addressing mode), low 5 bits into the CCR.
//! Timing: docs/specs/instr/MOVEtoCCR.md (`nn np np` after the operand read).

use crate::alu::{mode_rules, move_sr_hook, move_sr_plan};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of MOVEtoCCR.
pub const ID: InsnId = InsnId(63);

static RULES_ARR: [Rule; 11] = mode_rules::<11>(0x44c0, ID);

/// Decoding rules of MOVEtoCCR.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of MOVEtoCCR: operand read, `nn`, `np np`.
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    move_sr_plan(cpu, false)
}

/// Semantics of the hooks of MOVEtoCCR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    move_sr_hook(false, k, cpu);
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
    fn e10_07_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 + 40 + 5);
    }

    #[test]
    fn e10_07_decode_illegal() {
        for op in [0x44c8u16, 0x46c0, 0x44fd, 0x4E71] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e10_07_plan_basic() {
        assert_eq!(clocks(0x44c0, 0x2700), 12); // Dn
        assert_eq!(clocks(0x44d0, 0x2700), 16); // (A0)
        assert_eq!(clocks(0x44f9, 0x2700), 24); // (xxx).L
        assert_eq!(clocks(0x44fc, 0x2700), 16); // #imm
    }

    #[test]
    fn e10_07_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x44C0, 0, 0, 0xFFFF_001F, 0x2700, &mut r);
        assert_eq!((n, cpu.regs.pc, cpu.regs.sr), (12, 0x22, 0x271F));
        let mut r2 = ram(&[(0x1000, 0xFF05)]);
        let (cpu, n) = run(0x44D0, 0, 0x1000, 0, 0x2710, &mut r2);
        assert_eq!((n, cpu.regs.sr), (16, 0x2705));
        let (cpu, n) = run(0x44FC, 0x0003, 0, 0, 0x0000, &mut r);
        assert_eq!((n, cpu.regs.sr), (16, 0x0003));
    }
}
