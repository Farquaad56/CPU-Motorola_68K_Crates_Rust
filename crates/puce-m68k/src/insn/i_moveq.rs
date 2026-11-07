//! MOVEQ (`0111 rrr0 dddd dddd`): sign-extended 8-bit immediate into Dn, 4 clocks (`np`).
//! N and Z set, V and C cleared, X untouched. Timing: docs/specs/instr/MOVEQ.md.

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{Plan, Tok};
use crate::regs::{C, N, V, Z};

/// Identifier of MOVEQ.
pub const ID: InsnId = InsnId(4);

/// Decoding rules: `0111 rrr0 dddddddd` (bit 8 clear).
pub const RULES: &[Rule] = &[Rule {
    mask: 0xF100,
    value: 0x7000,
    id: ID,
    variant: 0,
}];

/// Plan: the register is loaded by the hook, then the next opcode is prefetched.
pub fn plan(_variant: u8) -> Plan {
    &[Tok::H(1), Tok::Pf]
}

/// Hook 1: `Dn = sign-extended data`, set N and Z, clear V and C.
pub fn hook(_k: u8, cpu: &mut Cpu) {
    let ird = cpu.lat.ird;
    let v = ird as u8 as i8 as i32 as u32;
    cpu.regs.d[((ird >> 9) & 7) as usize] = v;
    let mut sr = cpu.regs.sr & !(N | Z | V | C);
    if v == 0 {
        sr |= Z;
    }
    if v & 0x8000_0000 != 0 {
        sr |= N;
    }
    cpu.regs.sr = sr;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::decode::{decode, TABLE};
    use crate::engine::plan::plan_clocks;
    use crate::regs::X;
    use crate::state::CpuState;

    struct Ram;
    impl Bus for Ram {
        fn read(&mut self, _a: u32, _f: u8, _u: bool, _l: bool) -> u16 {
            0
        }
        fn write(&mut self, _a: u32, _f: u8, _u: bool, _l: bool, _d: u16) {}
    }

    fn run(op: u16) -> (Cpu, u32) {
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            d: [0x1234_5678; 8],
            sr: 0x2700 | X | V | C,
            pc: 0x100,
            prefetch: [op, 0],
            ..CpuState::default()
        });
        let n = cpu.step(&mut Ram);
        (cpu, n)
    }

    #[test]
    fn e06_04_decode_legal() {
        // 8 registers x 256 data values
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 8 * 256);
        assert_eq!(decode(0x7000).id, ID);
        assert_eq!(decode(0x7EFF).id, ID);
    }

    #[test]
    fn e06_04_decode_illegal() {
        // bit 8 set, neighbouring line 6 and 8
        for op in [0x7100u16, 0x7FFF, 0x6000, 0x8000] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e06_04_plan_basic() {
        assert_eq!(plan_clocks(plan(0)), 4);
    }

    #[test]
    fn e06_04_flags_sample() {
        // MOVEQ #0,D0 -> Z ; #$80 -> N and D0 = $FFFFFF80 ; #$7F -> none; X kept, V/C cleared
        for (op, d0, want) in [
            (0x7000u16, 0u32, Z),
            (0x7080, 0xFFFF_FF80, N),
            (0x707F, 0x7F, 0),
        ] {
            let (cpu, n) = run(op);
            assert_eq!(n, 4);
            assert_eq!(cpu.regs.d[0], d0);
            assert_eq!(cpu.regs.sr & 0x1F, X | want, "{op:#06x}");
            assert_eq!(cpu.regs.d[1], 0x1234_5678);
        }
    }
}
