//! BTST (`0000 rrr1 00 ea` and `0000 1000 00 ea` + bit number): bit test.
//! Z is set when the tested bit was zero; other flags untouched. Data register: bit modulo 32 (long);
//! memory: bit modulo 8 (byte). Timing: docs/specs/instr/BTST.md (variable idle cycle).

use crate::alu::{bit_hook, bit_plan, bit_rules, Bit, N_RULES_BTST};
use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::Plan;

/// Identifier of BTST.
pub const ID: InsnId = InsnId(42);

static RULES_ARR: [Rule; N_RULES_BTST] = bit_rules::<{ N_RULES_BTST }>(0, ID);

/// Decoding rules of BTST.
pub static RULES: &[Rule] = &RULES_ARR;

/// Plan of the instruction being executed (depends on its form: see `crate::alu`).
pub fn plan(_variant: u8, cpu: &Cpu) -> Plan {
    bit_plan(cpu, true)
}

/// Semantics of the hooks of BTST.
pub fn hook(k: u8, cpu: &mut Cpu) {
    bit_hook(Bit::Test, k, cpu);
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

    fn run(op: u16, d0: u32, d1: u32, sr: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [d0, d1, 0, 0, 0, 0, 0, 0],
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
    fn e08_13_decode_legal() {
        let n = TABLE.iter().filter(|d| d.id == ID).count();
        assert_eq!(n, 476);
    }

    #[test]
    fn e08_13_decode_illegal() {
        for op in [0x083C, 0x0108] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e08_13_plan_basic() {
        assert_eq!(clocks(0x0300), 4); // Dn,D0 (idle cycle added at run time)
        assert_eq!(clocks(0x0310), 8); // D1,(A0)
        assert_eq!(clocks(0x0800), 8); // #n,D0
        assert_eq!(clocks(0x0839), 20); // #n,(xxx).L
    }

    #[test]
    fn e08_13_flags_sample() {
        let (cpu, n) = run(0x0300, 0x10, 0x4, 0x1f);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x10, 0x1b));
        let (cpu, n) = run(0x0300, 0x10, 0x5, 0x1b);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x10, 0x1f));
        let (cpu, n) = run(0x0300, 0x0, 0x34, 0x0);
        assert_eq!((n, cpu.regs.d[0], cpu.regs.sr & 0x1F), (6, 0x0, 0x4));
    }
}
