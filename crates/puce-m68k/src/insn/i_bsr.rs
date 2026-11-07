//! BSR (`0110 0001 dddddddd`): pushes the return address (long, high word first) then branches.
//! No flags. Timing: docs/specs/instr/BSR.md (`n nS ns np np`).

use crate::cpu::Cpu;
use crate::decode::{InsnId, Rule};
use crate::engine::plan::{AddrSrc, Plan, Tok};

/// Identifier of BSR.
pub const ID: InsnId = InsnId(50);

const HK_SET: u8 = 0x20; // aux = return address, aux2 = target
const HK_PUSH: u8 = 0x21; // SP -= 4, ea = SP, val = aux
const HK_JUMP: u8 = 0x22; // PC = aux2

/// Decoding rules of BSR.
pub const RULES: &[Rule] = &[Rule {
    mask: 0xFF00,
    value: 0x6100,
    id: ID,
    variant: 0,
}];

static PLAN: [Tok; 7] = [
    Tok::N(2),
    Tok::H(HK_SET),
    Tok::H(HK_PUSH),
    Tok::Wl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::Pf,
];

/// Plan of BSR: `n nS ns np np`.
pub fn plan(_variant: u8, _cpu: &Cpu) -> Plan {
    &PLAN
}

/// Semantics of the hooks of BSR.
pub fn hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_SET => {
            let ird = cpu.lat.ird;
            cpu.lat.aux = if ird & 0xFF == 0 {
                cpu.regs.pc
            } else {
                cpu.regs.pc.wrapping_sub(2)
            };
            cpu.lat.aux2 = crate::insn::i_bcc::target(ird, cpu.regs.irc, cpu.regs.pc);
        }
        HK_PUSH => {
            let sp = cpu.regs.an(7).wrapping_sub(4);
            cpu.regs.set_an(7, sp);
            cpu.lat.ea = sp;
            cpu.lat.val = cpu.lat.aux;
        }
        HK_JUMP => cpu.regs.pc = cpu.lat.aux2,
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
    fn e09_02_decode_legal() {
        assert_eq!(TABLE.iter().filter(|d| d.id == ID).count(), 256);
    }

    #[test]
    fn e09_02_decode_illegal() {
        for op in [0x6000u16, 0x6200, 0x60FF, 0x62FF] {
            assert_ne!(decode(op).id, ID, "{op:#06x}");
        }
    }

    #[test]
    fn e09_02_plan_basic() {
        assert_eq!(plan_clocks(plan(0, &Cpu::new())), 18);
    }

    #[test]
    fn e09_02_flags_sample() {
        // BSR.B +4: target 0x1C + 2 + 4 = 0x22, return address 0x1E
        let mut r = ram(&[]);
        let (cpu, n) = run(0x6104, 0, 0, 0, 0x15, &mut r);
        assert_eq!(
            (n, cpu.regs.pc, cpu.regs.sr, cpu.regs.usp),
            (18, 0x26, 0x15, 0x3FC)
        );
        assert_eq!(r.writes, std::vec![(0x3FC, 0), (0x3FE, 0x1E)]);
        // BSR.W +0x10: target 0x1E + 0x10 = 0x2E, return address 0x20
        let mut r = ram(&[]);
        let (cpu, n) = run(0x6100, 0x0010, 0, 0, 0, &mut r);
        assert_eq!((n, cpu.regs.pc), (18, 0x32));
        assert_eq!(r.writes, std::vec![(0x3FC, 0), (0x3FE, 0x20)]);
        // BSR.B -2: target 0x1C
        let mut r = ram(&[]);
        let (cpu, _) = run(0x61FE, 0, 0, 0, 0, &mut r);
        assert_eq!(cpu.regs.pc, 0x20);
    }
}
