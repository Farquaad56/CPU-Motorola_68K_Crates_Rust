//! Exception processing shared by the instructions. Only the privilege violation (vector 8) is
//! needed so far; E11 adds the other vectors on the same frame sequence (group 1 and 2 frame:
//! PC low, SR, PC high, then the vector read and two prefetches).

use crate::cpu::Cpu;
use crate::engine::plan::{AddrSrc, Plan, Tok};
use crate::regs::{S, T};

/// Hook: saves SR and the PC of the faulting instruction, enters supervisor mode (T cleared).
pub const HK_PRIV: u8 = 0xF0;
/// Hook: address and data of the first push (PC low word).
pub const HK_PUSH_PC_LO: u8 = 0xF1;
/// Hook: address and data of the second push (old SR).
pub const HK_PUSH_SR: u8 = 0xF2;
/// Hook: address and data of the third push (PC high word); updates the supervisor stack pointer.
pub const HK_PUSH_PC_HI: u8 = 0xF3;
/// Hook: address of the exception vector.
pub const HK_VECTOR: u8 = 0xF4;
/// Hook: PC = vector read.
pub const HK_JUMP: u8 = 0xF5;

/// Vector number of the privilege violation.
pub const VEC_PRIVILEGE: u32 = 8;

static PRIVILEGE: [Tok; 14] = [
    Tok::N(4),
    Tok::H(HK_PRIV),
    Tok::H(HK_PUSH_PC_LO),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_SR),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_PUSH_PC_HI),
    Tok::Ww(AddrSrc::Ea),
    Tok::H(HK_VECTOR),
    Tok::Rl(AddrSrc::Ea),
    Tok::H(HK_JUMP),
    Tok::Pf,
    Tok::N(2),
    Tok::Pf,
];

/// Plan of the privilege violation exception (34 clocks): `nn ns ns ns nV nv np n np`.
pub fn privilege_violation() -> Plan {
    &PRIVILEGE
}

/// True when `k` is one of the exception hooks.
pub fn is_hook(k: u8) -> bool {
    (HK_PRIV..=HK_JUMP).contains(&k)
}

/// Semantics of the exception hooks. The faulting instruction is at `PC - 4` when the plan starts.
pub fn hook(k: u8, cpu: &mut Cpu) {
    match k {
        HK_PRIV => {
            cpu.lat.aux = cpu.regs.pc.wrapping_sub(4);
            cpu.lat.aux2 = u32::from(cpu.regs.sr);
            cpu.regs.sr = (cpu.regs.sr | S) & !T;
        }
        HK_PUSH_PC_LO => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(2);
            cpu.lat.val = cpu.lat.aux & 0xFFFF;
        }
        HK_PUSH_SR => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(6);
            cpu.lat.val = cpu.lat.aux2;
        }
        HK_PUSH_PC_HI => {
            cpu.lat.ea = cpu.regs.ssp.wrapping_sub(4);
            cpu.lat.val = cpu.lat.aux >> 16;
            cpu.regs.ssp = cpu.regs.ssp.wrapping_sub(6);
        }
        HK_VECTOR => cpu.lat.ea = VEC_PRIVILEGE * 4,
        HK_JUMP => cpu.regs.pc = cpu.lat.val,
        _ => {}
    }
}
