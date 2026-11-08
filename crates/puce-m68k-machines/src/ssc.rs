//! SingleStepTests/m68000 runner: reader of the `.json.bin` binary format (spec 33) and
//! single-test execution.

use crate::flat::FlatRam;
use puce_m68k::cpu::Cpu;
use puce_m68k::probe::BusEvent;
use puce_m68k::state::CpuState;
use std::path::Path;

/// Magic of a whole file.
const MAGIC_FILE: u32 = 0x1A3F_5D71;
/// Magic of one test.
const MAGIC_TEST: u32 = 0xABC1_2367;
/// Magic of the name block.
const MAGIC_NAME: u32 = 0x89AB_CDEF;
/// Magic of a state block.
const MAGIC_STATE: u32 = 0x0123_4567;
/// Magic of the transactions block.
const MAGIC_TRANS: u32 = 0x4567_89AB;

/// CPU and memory state of a test (initial or final).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// D0-D7.
    pub d: [u32; 8],
    /// A0-A6.
    pub a: [u32; 7],
    /// User stack pointer.
    pub usp: u32,
    /// Supervisor stack pointer.
    pub ssp: u32,
    /// Status register.
    pub sr: u32,
    /// Program counter (address of the next word to prefetch).
    pub pc: u32,
    /// Prefetch queue: IR then IRC.
    pub prefetch: [u32; 2],
    /// Words of memory touched by the test: (even address, 16-bit word).
    pub ram: Vec<(u32, u16)>,
}

/// Kind of a bus transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Internal cycle (`n`).
    Idle,
    /// Write (`w`).
    Write,
    /// Read (`r`).
    Read,
    /// TAS read-modify-write cycle (`t`).
    Tas,
    /// Read with address error (`re`).
    ReadAddrErr,
    /// Write with address error (`we`).
    WriteAddrErr,
}

/// One transaction of a test (fields `fc` to `lds` are 0 for `Idle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trans {
    /// Transaction kind.
    pub kind: Kind,
    /// Duration in clocks.
    pub cycles: u32,
    /// Function code.
    pub fc: u32,
    /// Address on the bus.
    pub addr: u32,
    /// Data on the bus.
    pub data: u32,
    /// UDS strobe.
    pub uds: u32,
    /// LDS strobe.
    pub lds: u32,
}

/// One test: initial state, expected final state and bus transactions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Test {
    /// Test name (disassembly of the instruction).
    pub name: String,
    /// Initial state.
    pub initial: State,
    /// Expected final state.
    pub final_: State,
    /// Expected bus transactions.
    pub trans: Vec<Trans>,
    /// Total duration in clocks.
    pub length: u32,
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.b.len())
            .ok_or_else(|| format!("file truncated at byte {}", self.pos))?;
        let s = &self.b[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

    fn magic(&mut self, want: u32, what: &str) -> Result<(), String> {
        let at = self.pos;
        let got = self.u32()?;
        if got == want {
            Ok(())
        } else {
            Err(format!(
                "bad {what} magic at byte {at}: {got:#010x}, expected {want:#010x}"
            ))
        }
    }

    fn block(&mut self, magic: u32, what: &str) -> Result<(), String> {
        let _numbytes = self.u32()?;
        self.magic(magic, what)
    }

    fn state(&mut self) -> Result<State, String> {
        self.block(MAGIC_STATE, "state")?;
        let mut s = State::default();
        for v in s.d.iter_mut() {
            *v = self.u32()?;
        }
        for v in s.a.iter_mut() {
            *v = self.u32()?;
        }
        s.usp = self.u32()?;
        s.ssp = self.u32()?;
        s.sr = self.u32()?;
        s.pc = self.u32()?;
        s.prefetch = [self.u32()?, self.u32()?];
        let n = self.u32()? as usize;
        // Each RAM entry takes 6 bytes: never reserve more than the file can hold.
        s.ram.reserve(n.min(self.remaining() / 6));
        for _ in 0..n {
            let addr = self.u32()?;
            let data = self.u16()?;
            s.ram.push((addr, data));
        }
        Ok(s)
    }

    fn trans(&mut self) -> Result<(u32, Vec<Trans>), String> {
        self.block(MAGIC_TRANS, "transactions")?;
        let length = self.u32()?;
        let n = self.u32()? as usize;
        let mut v = Vec::with_capacity(n.min(self.remaining() / 5));
        for _ in 0..n {
            let at = self.pos;
            let kind = match self.u8()? {
                0 => Kind::Idle,
                1 => Kind::Write,
                2 => Kind::Read,
                3 => Kind::Tas,
                4 => Kind::ReadAddrErr,
                5 => Kind::WriteAddrErr,
                k => return Err(format!("unknown transaction kind {k} at byte {at}")),
            };
            let cycles = self.u32()?;
            let mut t = Trans {
                kind,
                cycles,
                fc: 0,
                addr: 0,
                data: 0,
                uds: 0,
                lds: 0,
            };
            if kind != Kind::Idle {
                t.fc = self.u32()?;
                t.addr = self.u32()?;
                t.data = self.u32()?;
                t.uds = self.u32()?;
                t.lds = self.u32()?;
            }
            v.push(t);
        }
        Ok((length, v))
    }

    fn test(&mut self) -> Result<Test, String> {
        self.block(MAGIC_TEST, "test")?;
        self.block(MAGIC_NAME, "name")?;
        let len = self.u32()? as usize;
        let name = String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| "test name is not valid UTF-8".to_string())?;
        let initial = self.state()?;
        let final_ = self.state()?;
        let (length, trans) = self.trans()?;
        Ok(Test {
            name,
            initial,
            final_,
            trans,
            length,
        })
    }
}

/// Parses a whole `.json.bin` file. Never panics: a wrong magic or a truncated file is an `Err`.
pub fn parse_file(bytes: &[u8]) -> Result<Vec<Test>, String> {
    let mut r = Reader { b: bytes, pos: 0 };
    r.magic(MAGIC_FILE, "file")?;
    let n = r.u32()? as usize;
    let mut tests = Vec::with_capacity(n.min(4096));
    for i in 0..n {
        tests.push(r.test().map_err(|e| format!("test {i}: {e}"))?);
    }
    Ok(tests)
}

/// Result of running one test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The final state and the bus transactions match.
    Pass,
    /// Kept for the summary format: address error tests are no longer skipped (D-052).
    SkipAddrError,
    /// First difference found (names the test, the field, expected and obtained values).
    Fail(String),
}

fn kind_code(k: Kind) -> u8 {
    match k {
        Kind::Idle => 0,
        Kind::Write => 1,
        Kind::Read => 2,
        Kind::Tas => 3,
        Kind::ReadAddrErr => 4,
        Kind::WriteAddrErr => 5,
    }
}

/// Compares the expected transactions with the journal; the message names the transaction index.
fn compare_trans(expected: &[Trans], got: &[BusEvent]) -> Result<(), String> {
    for (i, e) in expected.iter().enumerate() {
        let Some(g) = got.get(i) else {
            return Err(format!(
                "transaction #{i}: expected {:?} of {} clocks, got nothing (journal has {} events)",
                e.kind,
                e.cycles,
                got.len()
            ));
        };
        let mut fields: Vec<(&str, u32, u32)> = vec![
            // The oracle labels the read of TAS as a plain read; the core tags it (kind 3).
            (
                "kind",
                u32::from(kind_code(e.kind)),
                u32::from(if g.kind == 3 { 2 } else { g.kind }),
            ),
            ("cycles", e.cycles, g.clocks),
        ];
        if e.kind != Kind::Idle {
            fields.extend([
                ("fc", e.fc, u32::from(g.fc)),
                ("addr", e.addr, g.addr),
                ("uds", e.uds, u32::from(g.uds)),
                ("lds", e.lds, u32::from(g.lds)),
            ]);
            // The data of an aborted cycle is whatever floated on the bus: not compared.
            if !matches!(e.kind, Kind::ReadAddrErr | Kind::WriteAddrErr) {
                fields.push(("data", e.data, u32::from(g.data)));
            }
        }
        for (name, want, have) in fields {
            if want != have {
                return Err(format!(
                    "transaction #{i}: {name}: expected {want:#x}, got {have:#x}"
                ));
            }
        }
    }
    if got.len() > expected.len() {
        return Err(format!(
            "transaction #{}: unexpected extra event ({} expected, {} obtained)",
            expected.len(),
            expected.len(),
            got.len()
        ));
    }
    Ok(())
}

fn compare_state(cpu: &CpuState, want: &State) -> Result<(), String> {
    let mut fields: Vec<(String, u32, u32)> = Vec::new();
    for i in 0..8 {
        fields.push((format!("d{i}"), want.d[i], cpu.d[i]));
    }
    for i in 0..7 {
        fields.push((format!("a{i}"), want.a[i], cpu.a[i]));
    }
    fields.push(("usp".into(), want.usp, cpu.usp));
    fields.push(("ssp".into(), want.ssp, cpu.ssp));
    fields.push(("sr".into(), want.sr, u32::from(cpu.sr)));
    fields.push(("pc".into(), want.pc, cpu.pc));
    fields.push((
        "prefetch0".into(),
        want.prefetch[0],
        u32::from(cpu.prefetch[0]),
    ));
    fields.push((
        "prefetch1".into(),
        want.prefetch[1],
        u32::from(cpu.prefetch[1]),
    ));
    for (name, w, h) in fields {
        if w != h {
            return Err(format!("{name}: expected {w:#x}, got {h:#x}"));
        }
    }
    Ok(())
}

/// Runs one test on `cpu` with `ram` as memory and compares the final state and the bus
/// transactions (spec 33, section 5).
pub fn run_test(cpu: &mut Cpu, ram: &mut FlatRam, t: &Test) -> Outcome {
    ram.clear_touched();
    for &(addr, word) in &t.initial.ram {
        ram.poke_word(addr, word);
    }
    let s = &t.initial;
    cpu.load_state(&CpuState {
        d: s.d,
        a: s.a,
        usp: s.usp,
        ssp: s.ssp,
        sr: s.sr as u16,
        pc: s.pc,
        prefetch: [s.prefetch[0] as u16, s.prefetch[1] as u16],
    });
    cpu.clear_journal();
    // Whole instructions: Cpu::step decodes IR and returns the clocks used (0 = illegal opcode,
    // nothing runs). A test of length 0 runs nothing.
    let mut clocks = 0u32;
    while clocks < t.length {
        let n = cpu.step(ram);
        if n == 0 {
            break;
        }
        clocks += n;
    }

    let fail = |what: String| Outcome::Fail(format!("{}: {what}", t.name));
    if let Err(e) = compare_state(&cpu.state(), &t.final_) {
        return fail(e);
    }
    for &(addr, word) in &t.final_.ram {
        let got = ram.peek_word(addr);
        if got != word {
            return fail(format!(
                "ram[{addr:#08x}]: expected {word:#06x}, got {got:#06x}"
            ));
        }
    }
    if clocks != t.length {
        return fail(format!("clocks: expected {}, got {clocks}", t.length));
    }
    if cpu.journal_overflow() > 0 {
        return fail(format!(
            "journal overflow ({} events lost)",
            cpu.journal_overflow()
        ));
    }
    match compare_trans(&t.trans, cpu.journal()) {
        Ok(()) => Outcome::Pass,
        Err(e) => fail(e),
    }
}

/// Per-file results of a run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileSummary {
    /// File name without `.json.bin` (for example `ADD.b`).
    pub stem: String,
    /// Number of tests in the file.
    pub total: u32,
    /// Tests that pass.
    pub passed: u32,
    /// Tests that fail.
    pub failed: u32,
    /// Tests skipped because they contain an address error (D-030).
    pub skipped_addr_error: u32,
    /// Messages of the first failures (5 at most).
    pub first_failures: Vec<String>,
}

/// Runs every test of a `.json.bin` file.
pub fn run_file(path: &Path, cpu: &mut Cpu, ram: &mut FlatRam) -> Result<FileSummary, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let tests = parse_file(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut s = FileSummary {
        stem: name.strip_suffix(".json.bin").unwrap_or(&name).to_string(),
        ..FileSummary::default()
    };
    for t in &tests {
        s.total += 1;
        match run_test(cpu, ram, t) {
            Outcome::Pass => s.passed += 1,
            Outcome::SkipAddrError => s.skipped_addr_error += 1,
            Outcome::Fail(m) => {
                s.failed += 1;
                if s.first_failures.len() < 5 {
                    s.first_failures.push(m);
                }
            }
        }
    }
    Ok(s)
}

/// Which test files to run (an empty list means all of them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Exact stems (`ADD.b`) or base names (`ADD` selects `ADD` and `ADD.*`).
    pub stems: Vec<String>,
}

impl Selection {
    /// Builds a selection from command line filters.
    pub fn from_filters(filters: &[String]) -> Self {
        Selection {
            stems: filters.to_vec(),
        }
    }

    /// Selects the stems of `epic` in `instructions.tsv` (columns `stem` and `epic`).
    pub fn from_epic(tsv: &Path, epic: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(tsv).map_err(|e| format!("{}: {e}", tsv.display()))?;
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
        let col = |n: &str| {
            header
                .iter()
                .position(|h| *h == n)
                .ok_or_else(|| format!("{}: no `{n}` column", tsv.display()))
        };
        let (c_stem, c_epic) = (col("stem")?, col("epic")?);
        let stems = lines
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f.get(c_epic) == Some(&epic)).then(|| f.get(c_stem).unwrap_or(&"").to_string())
            })
            .collect();
        Ok(Selection { stems })
    }

    /// True if the file `stem` is selected.
    pub fn matches(&self, stem: &str) -> bool {
        self.stems.is_empty()
            || self.stems.iter().any(|f| {
                stem == f
                    || stem
                        .strip_prefix(f.as_str())
                        .is_some_and(|r| r.starts_with('.'))
            })
    }
}

/// Runs the selected files of `dir` in alphabetical order. A requested stem without a file is an error.
pub fn run_dir(dir: &Path, sel: &Selection) -> Result<Vec<FileSummary>, String> {
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let p = e.map_err(|e| e.to_string())?.path();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(stem) = name.strip_suffix(".json.bin") {
            files.push((stem.to_string(), p.clone()));
        }
    }
    files.sort();
    for f in &sel.stems {
        let probe = Selection {
            stems: vec![f.clone()],
        };
        if !files.iter().any(|(s, _)| probe.matches(s)) {
            return Err(format!("no test file for `{f}` in {}", dir.display()));
        }
    }
    let mut cpu = Cpu::new();
    let mut ram = FlatRam::new();
    let mut out = Vec::new();
    for (stem, path) in &files {
        if sel.matches(stem) {
            out.push(run_file(path, &mut cpu, &mut ram)?);
        }
    }
    Ok(out)
}

/// Reads the known exceptions (first tab-separated column, empty lines ignored).
pub fn load_known(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.split('\t').next())
        .map(|s| s.trim().to_string())
        .collect())
}

/// True if the file has no failure, or if its base stem (before the first dot) is a known exception.
pub fn is_clean(s: &FileSummary, known: &[String]) -> bool {
    let base = s.stem.split('.').next().unwrap_or("");
    s.failed == 0 || known.iter().any(|k| k == base)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn p32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    fn enc_state(v: &mut Vec<u8>, s: &State) {
        p32(v, 0);
        p32(v, MAGIC_STATE);
        for x in s.d.iter().chain(s.a.iter()) {
            p32(v, *x);
        }
        for x in [s.usp, s.ssp, s.sr, s.pc, s.prefetch[0], s.prefetch[1]] {
            p32(v, x);
        }
        p32(v, s.ram.len() as u32);
        for (a, d) in &s.ram {
            p32(v, *a);
            v.extend_from_slice(&d.to_le_bytes());
        }
    }

    /// Minimal encoder: a file with the given tests.
    fn encode(tests: &[Test]) -> Vec<u8> {
        let mut v = Vec::new();
        p32(&mut v, MAGIC_FILE);
        p32(&mut v, tests.len() as u32);
        for t in tests {
            p32(&mut v, 0);
            p32(&mut v, MAGIC_TEST);
            p32(&mut v, 0);
            p32(&mut v, MAGIC_NAME);
            p32(&mut v, t.name.len() as u32);
            v.extend_from_slice(t.name.as_bytes());
            enc_state(&mut v, &t.initial);
            enc_state(&mut v, &t.final_);
            p32(&mut v, 0);
            p32(&mut v, MAGIC_TRANS);
            p32(&mut v, t.length);
            p32(&mut v, t.trans.len() as u32);
            for x in &t.trans {
                let k = match x.kind {
                    Kind::Idle => 0u8,
                    Kind::Write => 1,
                    Kind::Read => 2,
                    Kind::Tas => 3,
                    Kind::ReadAddrErr => 4,
                    Kind::WriteAddrErr => 5,
                };
                v.push(k);
                p32(&mut v, x.cycles);
                if x.kind != Kind::Idle {
                    for f in [x.fc, x.addr, x.data, x.uds, x.lds] {
                        p32(&mut v, f);
                    }
                }
            }
        }
        v
    }

    fn sample() -> Test {
        let initial = State {
            d: [1, 2, 3, 4, 5, 6, 7, 8],
            a: [10, 11, 12, 13, 14, 15, 16],
            usp: 0x1000,
            ssp: 0x2000,
            sr: 0x2704,
            pc: 0x404,
            prefetch: [0x4E71, 0x4E71],
            ram: vec![(0x400, 0x4E71), (0x402, 0x4E71)],
        };
        let mut final_ = initial.clone();
        final_.pc = 0x406;
        Test {
            name: "NOP".to_string(),
            initial,
            final_,
            trans: vec![
                Trans {
                    kind: Kind::Idle,
                    cycles: 2,
                    fc: 0,
                    addr: 0,
                    data: 0,
                    uds: 0,
                    lds: 0,
                },
                Trans {
                    kind: Kind::Read,
                    cycles: 4,
                    fc: 6,
                    addr: 0x404,
                    data: 0x4E71,
                    uds: 1,
                    lds: 1,
                },
            ],
            length: 6,
        }
    }

    #[test]
    fn e03_01_roundtrip() {
        let t = sample();
        let parsed = parse_file(&encode(std::slice::from_ref(&t))).unwrap();
        assert_eq!(parsed, vec![t]);
    }

    #[test]
    fn e03_01_bad_magic() {
        let mut b = encode(&[sample()]);
        b[0] ^= 0xFF;
        assert!(parse_file(&b).unwrap_err().contains("file magic"));
    }

    #[test]
    fn e03_01_truncated() {
        let b = encode(&[sample()]);
        for cut in 0..b.len() {
            assert!(parse_file(&b[..cut]).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn e03_01_kinds() {
        let kinds = [
            Kind::Idle,
            Kind::Write,
            Kind::Read,
            Kind::Tas,
            Kind::ReadAddrErr,
            Kind::WriteAddrErr,
        ];
        let mut t = sample();
        t.trans = kinds
            .iter()
            .map(|&kind| Trans {
                kind,
                cycles: 4,
                fc: if kind == Kind::Idle { 0 } else { 5 },
                addr: if kind == Kind::Idle { 0 } else { 0x10 },
                data: 0,
                uds: 0,
                lds: 0,
            })
            .collect();
        let mut bytes = encode(std::slice::from_ref(&t));
        let parsed = parse_file(&bytes).unwrap();
        let got: Vec<Kind> = parsed[0].trans.iter().map(|x| x.kind).collect();
        assert_eq!(got, kinds);
        // kind 6 is invalid: patch the first transaction (an `Idle`, 1 byte kind + 4 cycles).
        let first = bytes.len() - (5 + 5 * 25);
        bytes[first] = 6;
        assert!(parse_file(&bytes)
            .unwrap_err()
            .contains("unknown transaction kind 6"));
    }

    #[test]
    fn e03_01_ram_words() {
        let mut t = sample();
        t.initial.ram = vec![(0x00FF_FFFE, 0xBEEF), (0, 0x0102)];
        let parsed = parse_file(&encode(&[t])).unwrap();
        assert_eq!(
            parsed[0].initial.ram,
            vec![(0x00FF_FFFE, 0xBEEF), (0, 0x0102)]
        );
    }

    #[test]
    fn e03_01_real_nop() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/singlestep/NOP.json.bin"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let tests = parse_file(&bytes).unwrap();
        assert_eq!(tests.len(), 2500);
        assert!(tests.iter().all(|t| t.length > 0));
    }

    #[test]
    #[ignore]
    fn e03_01_scan_all() {
        // Statistics over every real file (used to settle the UNKNOWN points of spec 33).
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/singlestep");
        let (mut files, mut tests, mut min_bus, mut odd, mut idle_odd) =
            (0, 0u32, u32::MAX, 0u32, 0u32);
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if !p.to_string_lossy().ends_with(".json.bin") {
                continue;
            }
            files += 1;
            for t in parse_file(&std::fs::read(&p).unwrap()).unwrap() {
                tests += 1;
                for x in &t.trans {
                    if x.kind != Kind::Idle {
                        min_bus = min_bus.min(x.cycles);
                        if x.addr & 1 != 0 {
                            odd += 1;
                        }
                    } else if x.cycles % 2 != 0 {
                        idle_odd += 1;
                    }
                }
            }
        }
        std::println!("files={files} tests={tests} min_bus_cycles={min_bus} odd_addr_bus={odd} idle_odd={idle_odd}");
    }

    fn runner_state() -> State {
        State {
            d: [0; 8],
            a: [0; 7],
            usp: 0x1000,
            ssp: 0x2000,
            sr: 0x2700,
            pc: 0x404,
            prefetch: [0x4E71, 0x4E71],
            ram: vec![(0x400, 0x4E71)],
        }
    }

    /// A test an idle core passes: nothing to execute, final state = initial state.
    fn idle_test() -> Test {
        let s = runner_state();
        Test {
            name: "idle".to_string(),
            initial: s.clone(),
            final_: s,
            trans: vec![],
            length: 0,
        }
    }

    fn run(t: &Test) -> Outcome {
        run_test(&mut Cpu::new(), &mut FlatRam::new(), t)
    }

    #[test]
    fn e03_05_addr_error_compared() {
        let mut t = idle_test();
        t.trans = vec![Trans {
            kind: Kind::ReadAddrErr,
            cycles: 4,
            fc: 5,
            addr: 0x10,
            data: 0,
            uds: 1,
            lds: 1,
        }];
        // Address error cycles are compared like the others (no more skipped, D-052).
        assert!(matches!(run(&t), Outcome::Fail(_)));
    }

    #[test]
    fn e03_05_fail_reg() {
        let mut t = idle_test();
        t.final_.d[0] = 1;
        match run(&t) {
            Outcome::Fail(m) => assert!(
                m.contains("d0") && m.contains("expected 0x1, got 0x0"),
                "{m}"
            ),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn e03_05_fail_cycles() {
        let want = [
            Trans {
                kind: Kind::Idle,
                cycles: 2,
                fc: 0,
                addr: 0,
                data: 0,
                uds: 0,
                lds: 0,
            },
            Trans {
                kind: Kind::Read,
                cycles: 4,
                fc: 6,
                addr: 0x404,
                data: 0x4E71,
                uds: 1,
                lds: 1,
            },
        ];
        let ev = |kind, clocks| BusEvent {
            kind,
            clocks,
            fc: 6,
            addr: 0x404,
            data: 0x4E71,
            uds: true,
            lds: true,
        };
        assert_eq!(compare_trans(&want, &[ev(0, 2), ev(2, 4)]), Ok(()));
        let err = compare_trans(&want, &[ev(0, 2), ev(2, 6)]).unwrap_err();
        assert!(
            err.contains("transaction #1") && err.contains("cycles"),
            "{err}"
        );
        // a missing transaction names its index too
        let err = compare_trans(&want, &[ev(0, 2)]).unwrap_err();
        assert!(err.contains("transaction #1"), "{err}");
        // and so does the run of a real test whose core executes nothing
        let mut t = idle_test();
        t.trans = want.to_vec();
        match run(&t) {
            Outcome::Fail(m) => assert!(m.contains("transaction #0"), "{m}"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn e03_05_pass_synthetic() {
        assert_eq!(run(&idle_test()), Outcome::Pass);
    }

    #[test]
    fn e03_05_msg_names_test() {
        let mut t = idle_test();
        t.name = "007 NOP 4e71".to_string();
        t.final_.pc = 0x406;
        match run(&t) {
            Outcome::Fail(m) => assert!(m.starts_with("007 NOP 4e71: pc"), "{m}"),
            o => panic!("{o:?}"),
        }
    }

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("m68k_ssc_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn fake_dir(name: &str, stems: &[&str]) -> std::path::PathBuf {
        let d = tmp_dir(name);
        for s in stems {
            std::fs::write(d.join(format!("{s}.json.bin")), encode(&[idle_test()])).unwrap();
        }
        d
    }

    fn stems(r: &[FileSummary]) -> Vec<&str> {
        r.iter().map(|s| s.stem.as_str()).collect()
    }

    const ADD_FAMILY: [&str; 6] = ["ADD.b", "ADD.l", "ADD.w", "ADDA.l", "ADDX.b", "NOP"];

    #[test]
    fn e03_06_filter_exact() {
        let d = fake_dir("exact", &ADD_FAMILY);
        let sel = Selection::from_filters(&["ADD.b".to_string()]);
        let r = run_dir(&d, &sel).unwrap();
        assert_eq!(stems(&r), ["ADD.b"]);
        assert_eq!((r[0].total, r[0].passed), (1, 1));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn e03_06_filter_base() {
        let d = fake_dir("base", &ADD_FAMILY);
        let sel = Selection::from_filters(&["ADD".to_string()]);
        let r = run_dir(&d, &sel).unwrap();
        assert_eq!(stems(&r), ["ADD.b", "ADD.l", "ADD.w"]);
        let all = run_dir(&d, &Selection::default()).unwrap();
        assert_eq!(all.len(), 6);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn e03_06_epic_select() {
        let d = tmp_dir("epic");
        let tsv = d.join("i.tsv");
        std::fs::write(
            &tsv,
            "stem\tmnemonic\tsize\tfile\ttests\tepic\tnotes\nABCD\tABCD\t-\tABCD.json.bin\t2500\tE08\t\nADD.b\tADD\tb\tADD.b.json.bin\t2500\tE07\tn\nADDA.l\tADDA\tl\tADDA.l.json.bin\t2500\tE07\t\n",
        )
        .unwrap();
        let sel = Selection::from_epic(&tsv, "E07").unwrap();
        assert_eq!(sel.stems, ["ADD.b", "ADDA.l"]);
        assert!(sel.matches("ADD.b") && sel.matches("ADDA.l") && !sel.matches("ABCD"));
        assert!(Selection::from_epic(&d.join("absent.tsv"), "E07").is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn e03_06_known_parse() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/specs/data/sst_known_exceptions.txt"
        );
        let known = load_known(Path::new(path)).unwrap();
        // No known exception any more: every file of the oracle passes.
        assert!(known.is_empty());
    }

    #[test]
    fn e03_06_is_clean_known() {
        let known = vec!["TAS".to_string(), "TRAPV".to_string()];
        let s = |stem: &str, failed| FileSummary {
            stem: stem.to_string(),
            total: 2500,
            failed,
            ..FileSummary::default()
        };
        assert!(is_clean(&s("TAS", 5), &known));
        assert!(!is_clean(&s("ADD.b", 1), &known));
        assert!(is_clean(&s("ADD.b", 0), &known));
    }

    #[test]
    fn e03_06_missing_stem() {
        let d = fake_dir("missing", &ADD_FAMILY);
        let sel = Selection::from_filters(&["NOPE".to_string()]);
        let err = run_dir(&d, &sel).unwrap_err();
        assert!(err.contains("NOPE"), "{err}");
        let _ = std::fs::remove_dir_all(d);
    }
}
