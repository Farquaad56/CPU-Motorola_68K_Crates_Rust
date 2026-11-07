//! CMP.b / .w / .l (`1011 ddd0 ss ea`), CMPI (`0000 1100 ss ea` + immediate) and CMPM
//! (`1011 xxx1 ss 001 yyy`): `dst - src` flags only (X untouched). The SST files `CMP.*` also hold
//! CMPI and CMPM. Timing: docs/specs/instr/CMP.md.

use crate::alu::{cmp_hook, cmp_plan, cmp_rules, N_RULES_CMP};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of CMP.
pub const ID: InsnId = InsnId(24);

static RULES_ARR: [Rule; N_RULES_CMP] = cmp_rules(ID);

/// Decoding rules of CMP.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its operands: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    cmp_plan(cpu)
}

/// Semantics of the hooks of CMP.
pub fn hook(k: u8, cpu: &mut Cpu) {
    cmp_hook(k, cpu);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::regs::{C, N, V, X, Z};
    use crate::state::CpuState;

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16, d0: u32, d1: u32, a0: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, d1, 0, 0, 0, 0, 0, 0],
            a: [a0, 0, 0, 0, 0, 0, 0],
            sr,
            pc: 0x20,
            prefetch: [op, 0],
            ..CpuState::default()
        });
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    fn clocks(op: u16) -> u32 {
        let mut cpu = Cpu::new();
        cpu.lat.ird = op;
        plan_clocks(plan(0, &cpu))
    }

    #[test]
    fn e07_09_decode_legal() {
        // CMP per Dn: byte 53, word and long 61; CMPI 3 x 50; CMPM 3 sizes x 64 registers
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 8 * (53 + 2 * 61) + 3 * 50 + 3 * 64);
    }

    #[test]
    fn e07_09_decode_illegal() {
        // byte from An, EOR slot (opmode 4..6 with mode 0), CMPA slot (opmode 3)
        for op in [0xB008u16, 0xB100, 0xB140, 0xB0C0] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e07_09_plan_basic() {
        assert_eq!(clocks(0xB001), 4); // CMP.B D1,D0
        assert_eq!(clocks(0xB081), 6); // CMP.L D1,D0
        assert_eq!(clocks(0xB090), 14); // CMP.L (A0),D0
        assert_eq!(clocks(0x0C00), 8); // CMPI.B #,D0
        assert_eq!(clocks(0x0C80), 14); // CMPI.L #,D0
        assert_eq!(clocks(0x0C50), 12); // CMPI.W #,(A0)
        assert_eq!(clocks(0xB109), 12); // CMPM.B
        assert_eq!(clocks(0xB189), 20); // CMPM.L
    }

    #[test]
    fn e07_09_flags_sample() {
        let (cpu, n) = run(0xB001, 0x10, 0x20, 0, X); // CMP.B D1,D0: 0x10 - 0x20
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (4, 0x10, X | N | C));
        let (cpu, _) = run(0xB001, 5, 5, 0, N | V | C);
        assert_eq!(cpu.regs.sr & 0x1F, Z);
        let (cpu, _) = run(0xB081, 0x8000_0000, 1, 0, 0); // CMP.L D1,D0: overflow
        assert_eq!(cpu.regs.sr & 0x1F, V);
    }
}
