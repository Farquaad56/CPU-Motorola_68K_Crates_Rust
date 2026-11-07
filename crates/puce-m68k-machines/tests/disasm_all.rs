//! The disassembler and the decoding table must agree on the 65536 opcodes.

use puce_m68k::bus::Bus;
use puce_m68k::cpu::Cpu;
use puce_m68k::decode::{decode, InsnId};
use puce_m68k::disasm::{disasm, mnemonic};
use puce_m68k::ea::{ea_ext_words, Ea, Size};
use puce_m68k::state::CpuState;

const EXT_SETS: [[u16; 4]; 3] = [
    [0x1234, 0x5678, 0x9ABC, 0xDEF0],
    [0x0000, 0x0000, 0x0000, 0x0000],
    [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF],
];

fn text(op: u16, ext: &[u16; 4]) -> (String, usize) {
    let mut w = [0u16; 5];
    w[0] = op;
    w[1..].copy_from_slice(ext);
    let mut s = String::new();
    let n = disasm(&w, 0x1000, &mut s);
    (s, n)
}

/// Valid 68000 instructions the core does not decode yet (execution deferred, D-043).
fn deferred_text(s: &str) -> bool {
    [
        "illegal", "reset", "stop #", "rte", "trapv", "trap #", "chk.w ",
    ]
    .iter()
    .any(|p| s.starts_with(p))
}

#[test]
fn e12_05_no_panic() {
    for ext in &EXT_SETS {
        for op in 0..=u16::MAX {
            let (s, n) = text(op, ext);
            assert!(!s.is_empty() && (1..=5).contains(&n), "{op:#06x}");
        }
    }
    // short slices must not panic either
    let mut s = String::new();
    for op in 0..=u16::MAX {
        s.clear();
        assert!(disasm(&[op], 0, &mut s) >= 1);
    }
    assert!(disasm(&[], 0, &mut s) >= 1);
}

#[test]
fn e12_05_illegal_matches_decode() {
    let mut deferred = 0;
    for op in 0..=u16::MAX {
        let (s, _) = text(op, &EXT_SETS[0]);
        if decode(op).id == InsnId::ILLEGAL {
            if !s.starts_with("dc.w ") {
                assert!(deferred_text(&s), "{op:#06x}: {s}");
                deferred += 1;
            } else {
                assert_eq!(s, format!("dc.w ${op:04x}"));
            }
        } else {
            assert!(!s.starts_with("dc.w"), "{op:#06x}: {s}");
        }
    }
    // illegal + reset + stop + rte + trapv + 16 trap + 8 * 53 chk
    assert_eq!(deferred, 5 + 16 + 8 * 53);
}

#[test]
fn e12_05_mnemonic_prefix() {
    for op in 0..=u16::MAX {
        let id = decode(op).id;
        if id != InsnId::ILLEGAL {
            let (s, _) = text(op, &EXT_SETS[0]);
            assert!(
                s.starts_with(mnemonic(id)),
                "{op:#06x}: {s} / {}",
                mnemonic(id)
            );
        }
    }
}

struct Prog {
    fetches: u32,
}

impl Bus for Prog {
    fn read(&mut self, addr: u32, fc: u8, _uds: bool, _lds: bool) -> u16 {
        // Instruction fetches follow PC from 0x104; PC-relative operands are far away.
        if (fc == 2 || fc == 6) && addr < 0x120 {
            self.fetches += 1;
        }
        0x1234
    }
    fn write(&mut self, _addr: u32, _fc: u8, _uds: bool, _lds: bool, _data: u16) {}
}

/// Length of the control-flow instructions, whose plans fetch from another address.
fn flow_len(op: u16, id: u16) -> usize {
    let ea = |size| {
        let ea = Ea::decode(((op >> 3) & 7) as u8, (op & 7) as u8).unwrap();
        usize::from(ea_ext_words(ea, size))
    };
    match id {
        49 | 50 => 1 + usize::from(op & 0xFF == 0),
        51 => 2,
        52 => 1 + ea(Size::Byte),
        53 | 54 => 1 + ea(Size::Long),
        _ => 1,
    }
}

#[test]
fn e12_05_length_matches() {
    for op in 0..=u16::MAX {
        let id = decode(op).id;
        if id == InsnId::ILLEGAL {
            continue;
        }
        let (s, n) = text(op, &EXT_SETS[0]);
        if (49..=56).contains(&id.0) {
            assert_eq!(n, flow_len(op, id.0), "{op:#06x}: {s}");
            continue;
        }
        // ANDI/ORI/EORI to CCR/SR: two words. Their plan and MOVE to CCR/SR re-read the PC once.
        if (57..=62).contains(&id.0) {
            assert_eq!(n, 2, "{op:#06x}: {s}");
            continue;
        }
        let extra = u32::from((63..=64).contains(&id.0));
        // The plan of the instruction prefetches one word per word of the instruction.
        let mut cpu = Cpu::new();
        cpu.load_state(&CpuState {
            a: [0x2000, 0x2100, 0x2200, 0x2300, 0x2400, 0x2500, 0x2600],
            d: [0; 8],
            usp: 0x3000,
            ssp: 0x3800,
            sr: 0x2000,
            pc: 0x104,
            prefetch: [op, 0x1234],
        });
        let mut bus = Prog { fetches: 0 };
        cpu.step(&mut bus);
        assert_eq!(n as u32 + extra, bus.fetches, "{op:#06x}: {s}");
    }
}
