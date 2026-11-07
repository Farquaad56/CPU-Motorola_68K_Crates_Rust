//! Sub-command implementations (all stubs for now).

use std::path::PathBuf;

/// `m68k run <rom>`
pub fn run(_rom: &PathBuf) -> Result<(), String> {
    Err("not implemented".into())
}

/// `m68k ssc <dir> [--filter NAME]`
pub fn ssc(_dir: &PathBuf, _filter: Option<&str>) -> Result<(), String> {
    Err("not implemented".into())
}

/// `m68k disasm <file> --org ADDR`
pub fn disasm(_file: &PathBuf, _org: &str) -> Result<(), String> {
    Err("not implemented".into())
}

/// `m68k fetch`
pub fn fetch() -> Result<(), String> {
    Err("not implemented".into())
}
