//! Sub-command implementations (`ssc` is real, the others are still stubs).

use puce_m68k::cpu::Cpu;
use puce_m68k_machines::flat::FlatRam;
use puce_m68k_machines::ssc::{self, FileSummary, Selection};
use std::path::{Path, PathBuf};

/// Default known exceptions file.
const DEFAULT_KNOWN: &str = "docs/specs/data/sst_known_exceptions.txt";
/// Instruction table used by `--epic`.
const INSTRUCTIONS_TSV: &str = "docs/specs/data/instructions.tsv";

/// `m68k run <rom>`
pub fn run(_rom: &PathBuf) -> Result<(), String> {
    Err("not implemented".into())
}

/// Arguments of `m68k ssc`.
pub struct SscArgs<'a> {
    /// Directory of `*.json.bin` files.
    pub dir: &'a Path,
    /// `--filter` values.
    pub filters: &'a [String],
    /// `--epic` value.
    pub epic: Option<&'a str>,
    /// `--strict`.
    pub strict: bool,
    /// `--max-fail`.
    pub max_fail: Option<u32>,
    /// `--known`.
    pub known: Option<&'a Path>,
}

/// Exit code of `ssc`: with `strict`, 1 if a file that is not a known exception fails.
pub fn exit_code(summaries: &[FileSummary], known: &[String], strict: bool) -> i32 {
    i32::from(strict && summaries.iter().any(|s| !ssc::is_clean(s, known)))
}

fn line(s: &FileSummary) -> String {
    format!(
        "{} total={} pass={} fail={} skip_addr={}",
        s.stem, s.total, s.passed, s.failed, s.skipped_addr_error
    )
}

/// `m68k ssc <dir> [--filter F]... [--epic E] [--strict] [--max-fail N] [--known FILE]`
pub fn ssc(a: &SscArgs) -> Result<i32, String> {
    let mut sel = Selection::from_filters(a.filters);
    if let Some(epic) = a.epic {
        let e = Selection::from_epic(Path::new(INSTRUCTIONS_TSV), epic)?;
        if e.stems.is_empty() {
            return Err(format!(
                "no instruction for epic {epic} in {INSTRUCTIONS_TSV}"
            ));
        }
        sel.stems.extend(e.stems);
    }
    let known = match a.known {
        Some(p) => ssc::load_known(p)?,
        None if Path::new(DEFAULT_KNOWN).exists() => ssc::load_known(Path::new(DEFAULT_KNOWN))?,
        None => Vec::new(),
    };

    // Files in alphabetical order; a requested stem without a file is an error.
    let rd = std::fs::read_dir(a.dir).map_err(|e| format!("{}: {e}", a.dir.display()))?;
    let mut files: Vec<(String, PathBuf)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some((name.strip_suffix(".json.bin")?.to_string(), p))
        })
        .collect();
    files.sort();
    for f in &sel.stems {
        if !files
            .iter()
            .any(|(s, _)| Selection::from_filters(std::slice::from_ref(f)).matches(s))
        {
            return Err(format!("no test file for `{f}` in {}", a.dir.display()));
        }
    }

    let mut cpu = Cpu::new();
    let mut ram = FlatRam::new();
    let mut summaries = Vec::new();
    let mut failed_total = 0u32;
    for (stem, path) in &files {
        if !sel.matches(stem) {
            continue;
        }
        let s = ssc::run_file(path, &mut cpu, &mut ram)?;
        println!("{}", line(&s));
        for m in &s.first_failures {
            println!("    {m}");
        }
        failed_total += s.failed;
        summaries.push(s);
        if a.max_fail.is_some_and(|n| failed_total >= n) {
            println!("stopped: --max-fail {} reached", a.max_fail.unwrap_or(0));
            break;
        }
    }
    let total = FileSummary {
        stem: "TOTAL".into(),
        total: summaries.iter().map(|s| s.total).sum(),
        passed: summaries.iter().map(|s| s.passed).sum(),
        failed: failed_total,
        skipped_addr_error: summaries.iter().map(|s| s.skipped_addr_error).sum(),
        first_failures: Vec::new(),
    };
    println!("{}", line(&total));
    Ok(exit_code(&summaries, &known, a.strict))
}

/// `m68k disasm <file> --org ADDR`
pub fn disasm(_file: &PathBuf, _org: &str) -> Result<(), String> {
    Err("not implemented".into())
}

/// `m68k fetch`
pub fn fetch() -> Result<(), String> {
    Err("not implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(stem: &str, failed: u32) -> FileSummary {
        FileSummary {
            stem: stem.into(),
            total: 2500,
            passed: 2500 - failed,
            failed,
            ..FileSummary::default()
        }
    }

    #[test]
    fn e03_07_strict_exit() {
        let known = vec!["TAS".to_string()];
        assert_eq!(exit_code(&[s("ADD.b", 1), s("TAS", 5)], &known, true), 1);
        assert_eq!(exit_code(&[s("ADD.b", 0), s("TAS", 5)], &known, true), 0);
        assert_eq!(exit_code(&[s("ADD.b", 9)], &known, false), 0);
    }
}
