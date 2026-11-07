//! TAS (`0x4AC0 | ea`): tests the byte (N, Z; V and C cleared) then sets its bit 7. Memory forms use the
//! indivisible read-modify-write cycle. Timing: docs/specs/instr/TAS.md (SST stem is a known exception).

use crate::alu::{mode_rules, tas_hook, tas_plan};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of TAS.
pub const ID: InsnId = InsnId(68);

static RULES_ARR: [Rule; 8] = mode_rules::<8>(0x4AC0, ID);

/// Decoding rules of TAS.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of TAS: `np` on Dn, `nr n nw np` on memory (indivisible read-modify-write).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    tas_plan(cpu)
}

/// Semantics of the hooks of TAS.
pub fn hook(k: u8, cpu: &mut Cpu) {
    tas_hook(k, cpu);
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
    fn e10_12_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 + 40 + 2);
    }

    #[test]
    fn e10_12_decode_illegal() {
        for op in [0x4AC8u16, 0x4AFA, 0x4AFC, 0x4A40] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e10_12_plan_basic() {
        assert_eq!(clocks(0x4AC0, 0x2700), 4); // Dn
        assert_eq!(clocks(0x4AD0, 0x2700), 14); // (A0)
        assert_eq!(clocks(0x4AF9, 0x2700), 22); // (xxx).L
    }

    #[test]
    fn e10_12_flags_sample() {
        let mut r = ram(&[]);
        let (cpu, n) = run(0x4AC0, 0, 0, 0x1234_5600, 0x2710, &mut r); // zero byte
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr), (4, 0x1234_5680, 0x2714));
        let (cpu, _) = run(0x4AC0, 0, 0, 0xFF, 0x2703, &mut r); // negative byte
        assert_eq!((cpu.regs.d[0], cpu.regs.sr), (0xFF, 0x2708));
        let (cpu, _) = run(0x4AC0, 0, 0, 0x12, 0x271F, &mut r);
        assert_eq!((cpu.regs.d[0], cpu.regs.sr), (0x92, 0x2710));
    }
}
