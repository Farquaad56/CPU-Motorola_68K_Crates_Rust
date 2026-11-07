//! Sub-command implementations.

use puce_m68k::cpu::Cpu;
use puce_m68k::disasm::disasm as dis;
use puce_m68k::state::CpuState;
use puce_m68k_machines::flat::FlatRam;
use puce_m68k_machines::ssc::{self, FileSummary, Selection};
use std::path::{Path, PathBuf};

/// Default known exceptions file.
const DEFAULT_KNOWN: &str = "docs/specs/data/sst_known_exceptions.txt";
/// Instruction table used by `--epic`.
const INSTRUCTIONS_TSV: &str = "docs/specs/data/instructions.tsv";

/// Parses an address in hexadecimal, with or without `0x` or `$`.
pub fn parse_org(s: &str) -> Result<u32, String> {
    let t = s.trim();
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .or_else(|| t.strip_prefix('$'))
        .unwrap_or(t);
    u32::from_str_radix(t, 16).map_err(|_| format!("invalid address `{s}` (hexadecimal expected)"))
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn words_of(data: &[u8]) -> Vec<u16> {
    data.chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c.get(1).copied().unwrap_or(0)]))
        .collect()
}

/// Result of `m68k run`.
pub struct RunReport {
    /// Instructions executed.
    pub instructions: u64,
    /// Clocks consumed.
    pub clocks: u64,
    /// True when the program ended on an opcode the core does not run (STOP, illegal).
    pub stopped: bool,
    /// Final state (`pc` is the address of the next word to prefetch).
    pub state: CpuState,
}

/// Loads `data` at `org` (big-endian words, an odd length is padded with 0).
pub fn load_image(ram: &mut FlatRam, data: &[u8], org: u32) {
    for (i, w) in words_of(data).into_iter().enumerate() {
        ram.poke_word(org.wrapping_add(2 * i as u32), w);
    }
}

fn long_at(ram: &FlatRam, a: u32) -> u32 {
    (u32::from(ram.peek_word(a)) << 16) | u32::from(ram.peek_word(a.wrapping_add(2)))
}

/// Runs an image on the flat RAM until an opcode that does not run, or `max_instr` instructions.
/// Without `org` the image is loaded at 0 and SSP / PC come from vectors 0 and 1; with `org`
/// it is loaded there, PC = `org` and SSP = 0xFFFF00.
pub fn run_image(data: &[u8], max_instr: u64, org: Option<u32>) -> RunReport {
    let mut ram = FlatRam::new();
    load_image(&mut ram, data, org.unwrap_or(0));
    let (ssp, pc) = match org {
        Some(o) => (0x00FF_FF00, o),
        None => (long_at(&ram, 0), long_at(&ram, 4)),
    };
    let mut cpu = Cpu::new();
    cpu.load_state(&CpuState {
        a: [0; 7],
        d: [0; 8],
        usp: 0,
        ssp,
        sr: 0x2700,
        pc: pc.wrapping_add(4),
        prefetch: [ram.peek_word(pc), ram.peek_word(pc.wrapping_add(2))],
    });
    let mut rep = RunReport {
        instructions: 0,
        clocks: 0,
        stopped: false,
        state: cpu.state(),
    };
    while rep.instructions < max_instr {
        let n = cpu.step(&mut ram);
        if n == 0 {
            rep.stopped = true;
            break;
        }
        rep.instructions += 1;
        rep.clocks += u64::from(n);
    }
    rep.state = cpu.state();
    rep
}

/// Text of the final report of `m68k run`.
pub fn format_report(r: &RunReport) -> String {
    let s = &r.state;
    let mut out = String::new();
    for (i, d) in s.d.iter().enumerate() {
        out += &format!("D{i}={d:08X}{}", if i % 4 == 3 { "\n" } else { " " });
    }
    let a7 = if s.sr & 0x2000 != 0 { s.ssp } else { s.usp };
    for i in 0..8 {
        let a = if i == 7 { a7 } else { s.a[i] };
        out += &format!("A{i}={a:08X}{}", if i % 4 == 3 { "\n" } else { " " });
    }
    out += &format!(
        "SR={:04X} PC={:08X} USP={:08X} SSP={:08X}\n",
        s.sr,
        s.pc.wrapping_sub(4),
        s.usp,
        s.ssp
    );
    out += &format!(
        "instructions={} clocks={} {}\n",
        r.instructions,
        r.clocks,
        if r.stopped {
            "stopped (STOP or illegal opcode)"
        } else {
            "max-instr reached"
        }
    );
    out
}

/// `m68k run <bin> [--max-instr N] [--org HEX]`
pub fn run(rom: &Path, max_instr: u64, org: Option<&str>) -> Result<(), String> {
    let org = org.map(parse_org).transpose()?;
    let data = read_file(rom)?;
    print!("{}", format_report(&run_image(&data, max_instr, org)));
    Ok(())
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

/// Lines `addr: bytes  text` of the instructions found in `data` (loaded at `org`).
pub fn disasm_lines(data: &[u8], org: u32, count: Option<usize>) -> Vec<String> {
    let words = words_of(data);
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() && count.is_none_or(|c| out.len() < c) {
        let addr = org.wrapping_add(2 * i as u32);
        let mut text = String::new();
        let n = dis(&words[i..], addr, &mut text).min(words.len() - i);
        let bytes: Vec<String> = words[i..i + n].iter().map(|w| format!("{w:04x}")).collect();
        out.push(format!("{addr:06x}: {}  {text}", bytes.join(" ")));
        i += n;
    }
    out
}

/// `m68k disasm <bin> --org HEX [--count N]`
pub fn disasm(file: &Path, org: &str, count: Option<usize>) -> Result<(), String> {
    let org = parse_org(org)?;
    let data = read_file(file)?;
    for l in disasm_lines(&data, org, count) {
        println!("{l}");
    }
    Ok(())
}

/// Arguments of the `python docs/tools/fetch_refs.py` sub-process.
pub fn fetch_args(only: Option<&str>) -> Vec<String> {
    let mut v = vec!["docs/tools/fetch_refs.py".to_string()];
    if let Some(o) = only {
        v.push("--only".into());
        v.push(o.into());
    }
    v
}

/// `m68k fetch [--only docs|tests]`: runs the download script and returns its exit code.
pub fn fetch(only: Option<&str>) -> Result<i32, String> {
    let args = fetch_args(only);
    for py in ["python", "python3"] {
        match std::process::Command::new(py).args(&args).status() {
            Ok(s) => return Ok(s.code().unwrap_or(1)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{py}: {e}")),
        }
    }
    Err("python not found (tried `python` and `python3`)".into())
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
    fn e12_06_run_nop_loop() {
        // nop ; bra.s -4
        let img = [0x4E, 0x71, 0x60, 0xFC];
        let r = run_image(&img, 10, Some(0x1000));
        assert_eq!(r.instructions, 10);
        assert!(!r.stopped);
        assert_eq!(r.clocks, 5 * 4 + 5 * 10);
        assert!(format_report(&r).contains("instructions=10"));
        // stops on an opcode that does not run (0xA000)
        let r = run_image(&[0xA0, 0x00], 10, Some(0x1000));
        assert!(r.stopped && r.instructions == 0);
        // vectors: SSP 0x800, PC 0x100 (moveq #5,d0)
        let mut img = vec![0u8; 0x104];
        img[2] = 0x08;
        img[6] = 0x01;
        img[0x100] = 0x70;
        img[0x101] = 0x05;
        let r = run_image(&img, 1, None);
        assert_eq!((r.state.d[0], r.state.ssp), (5, 0x800));
    }

    #[test]
    fn e12_06_disasm_lines() {
        let img = [0x70, 0x01, 0x4E, 0x71, 0x4E, 0x75];
        let l = disasm_lines(&img, 0x1000, None);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0], "001000: 7001  moveq #1,d0");
        assert_eq!(l[2], "001004: 4e75  rts");
        assert_eq!(disasm_lines(&img, 0, Some(2)).len(), 2);
        let l = disasm_lines(&[0x4E, 0xB9, 0x00, 0x00, 0x12, 0x34], 0, None);
        assert_eq!(l, ["000000: 4eb9 0000 1234  jsr $1234.l"]);
    }

    #[test]
    fn e12_06_bad_file() {
        let e = run(Path::new("no/such/file.bin"), 1, None).unwrap_err();
        assert!(e.contains("no/such/file.bin"), "{e}");
        assert!(disasm(Path::new("no/such/file.bin"), "0", None).is_err());
    }

    #[test]
    fn e12_06_org_parse() {
        assert_eq!(parse_org("0x1000"), Ok(0x1000));
        assert_eq!(parse_org("1000"), Ok(0x1000));
        assert_eq!(parse_org("$ff"), Ok(0xFF));
        assert!(parse_org("zz").is_err());
    }

    #[test]
    fn e12_07_cmd_args() {
        assert_eq!(fetch_args(None), ["docs/tools/fetch_refs.py"]);
        assert_eq!(
            fetch_args(Some("docs")),
            ["docs/tools/fetch_refs.py", "--only", "docs"]
        );
    }

    #[test]
    fn e03_07_strict_exit() {
        let known = vec!["TAS".to_string()];
        assert_eq!(exit_code(&[s("ADD.b", 1), s("TAS", 5)], &known, true), 1);
        assert_eq!(exit_code(&[s("ADD.b", 0), s("TAS", 5)], &known, true), 0);
        assert_eq!(exit_code(&[s("ADD.b", 9)], &known, false), 0);
    }
}
