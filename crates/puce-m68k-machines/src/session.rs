//! Simulation session: simulation thread and state snapshots for the front end.
//!
//! [`Machine`] holds the CPU, the flat RAM and the debugger state and executes the protocol
//! commands synchronously (`Machine::handle`); [`Session`] runs a `Machine` in a thread and
//! exchanges [`Command`]s and JSON messages over `std::sync::mpsc` channels.

use crate::flat::FlatRam;
use crate::snapshot::{esc, BusRow, DisRow, Snapshot};
use crate::ssc::{self, Kind};
use puce_m68k::bus::Bus;
use puce_m68k::cpu::Cpu;
use puce_m68k::disasm::disasm;
use puce_m68k::probe::BusEvent;
use puce_m68k::state::CpuState;
use std::collections::VecDeque;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Instructions per second of the `speed` indexes (0 = as fast as possible).
pub const SPEEDS: [u32; 8] = [1, 4, 15, 60, 300, 3000, 60000, 0];
/// A byte written here (even address: high lane, odd: low lane) is appended to the console.
pub const CONSOLE_ADDR: u32 = 0x00FF_FF00;
/// Instructions of the trace kept.
const TRACE_LEN: usize = 64;
/// Bus cycles of the journal kept.
const BUSLOG_LEN: usize = 32;
/// Written addresses kept.
const WLOG_LEN: usize = 64;
/// Minimum interval between two states during `run`.
const STATE_INTERVAL: Duration = Duration::from_millis(34);

/// A command of the protocol (client to server).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `hello`
    Hello,
    /// `list_roms`
    ListRoms,
    /// `load`: a binary of the roms directory.
    Load {
        /// Relative path of the `.bin`.
        name: String,
    },
    /// `upload`: bytes sent by the page.
    Upload {
        /// Name shown for the binary.
        name: String,
        /// The decoded bytes.
        data: Vec<u8>,
    },
    /// `run`
    Run,
    /// `pause`
    Pause,
    /// `step`: `n` instructions.
    Step {
        /// Number of instructions.
        n: u64,
    },
    /// `step_clock`: `n` clocks.
    StepClock {
        /// Number of clocks.
        n: u64,
    },
    /// `reset`
    Reset,
    /// `speed`
    Speed {
        /// Index into [`SPEEDS`].
        idx: u8,
    },
    /// `bp_add`
    BpAdd {
        /// Address of the breakpoint.
        addr: u32,
    },
    /// `bp_del`
    BpDel {
        /// Address of the breakpoint.
        addr: u32,
    },
    /// `view`: observation window (absent fields are unchanged).
    View {
        /// Memory window base.
        mem_base: Option<u32>,
        /// Disassembly rows.
        dis_rows: Option<usize>,
        /// Trace rows.
        trace_rows: Option<usize>,
        /// Stack rows.
        stack_rows: Option<usize>,
        /// `pc`, `sp`, `an0`..`an6` or `man`.
        follow: Option<String>,
    },
    /// `sst_list`
    SstList,
    /// `sst_load`
    SstLoad {
        /// File stem (`NOP`, `ADD.b`).
        stem: String,
        /// Index of the test in the file.
        index: usize,
    },
    /// `console_clear`
    ConsoleClear,
}

/// RAM plus the console and the write log.
struct DebugBus {
    ram: FlatRam,
    console: String,
    wlog: VecDeque<u32>,
}

impl Bus for DebugBus {
    fn read(&mut self, addr: u32, fc: u8, uds: bool, lds: bool) -> u16 {
        self.ram.read(addr, fc, uds, lds)
    }

    fn write(&mut self, addr: u32, fc: u8, uds: bool, lds: bool, data: u16) {
        self.ram.write(addr, fc, uds, lds, data);
        let a = addr & 0x00FF_FFFE;
        if a == CONSOLE_ADDR {
            if uds {
                self.console.push(char::from((data >> 8) as u8));
            } else if lds {
                self.console.push(char::from(data as u8));
            }
        }
        if self.wlog.len() == WLOG_LEN {
            self.wlog.pop_front();
        }
        self.wlog.push_back(a);
    }
}

/// What the memory window follows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Follow {
    Pc,
    Sp,
    An(usize),
    Manual,
}

struct View {
    mem_base: u32,
    dis_rows: usize,
    trace_rows: usize,
    stack_rows: usize,
    follow: Follow,
}

/// Test being debugged cycle by cycle (`sst_load`).
struct SstDebug {
    name: String,
    index: usize,
    expected: Vec<ssc::Trans>,
    actual: Vec<BusRow>,
}

/// The simulated machine and the state of the debugger.
pub struct Machine {
    cpu: Cpu,
    bus: DebugBus,
    roms_dir: PathBuf,
    rom: Option<(String, Vec<u8>)>,
    bps: Vec<u32>,
    instrs: u64,
    clocks: u64,
    status: &'static str,
    speed: u8,
    trace: VecDeque<(u32, String)>,
    buslog: VecDeque<BusRow>,
    view: View,
    ips: f64,
    mhz: f64,
    sst: Option<SstDebug>,
}

fn row(e: &BusEvent) -> BusRow {
    BusRow {
        kind: e.kind,
        addr: e.addr,
        data: e.data,
        fc: e.fc,
        uds: e.uds,
        lds: e.lds,
        clocks: e.clocks,
    }
}

fn kind_str(k: Kind) -> &'static str {
    match k {
        Kind::Idle => "n",
        Kind::Write => "w",
        Kind::Read => "r",
        Kind::Tas => "t",
        Kind::ReadAddrErr => "re",
        Kind::WriteAddrErr => "we",
    }
}

fn error_json(msg: &str) -> String {
    format!("{{\"type\":\"error\",\"message\":\"{}\"}}", esc(msg))
}

/// True for a relative path made of plain components (no `..`, no root, no prefix).
fn safe_relative(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

impl Machine {
    /// A machine with no binary loaded; `roms_dir` is where `list_roms` and `load` look.
    pub fn new(roms_dir: PathBuf) -> Machine {
        let mut m = Machine {
            cpu: Cpu::new(),
            bus: DebugBus {
                ram: FlatRam::new(),
                console: String::new(),
                wlog: VecDeque::new(),
            },
            roms_dir,
            rom: None,
            bps: Vec::new(),
            instrs: 0,
            clocks: 0,
            status: "idle",
            speed: 5,
            trace: VecDeque::new(),
            buslog: VecDeque::new(),
            view: View {
                mem_base: 0,
                dis_rows: 40,
                trace_rows: 16,
                stack_rows: 14,
                follow: Follow::Pc,
            },
            ips: 0.0,
            mhz: 0.0,
            sst: None,
        };
        m.reset_registers();
        m
    }

    fn long_at(&self, a: u32) -> u32 {
        (u32::from(self.bus.ram.peek_word(a)) << 16)
            | u32::from(self.bus.ram.peek_word(a.wrapping_add(2)))
    }

    /// Registers from vectors 0 and 1 (no RESET exception), counters and logs cleared.
    fn reset_registers(&mut self) {
        let (ssp, pc) = (self.long_at(0), self.long_at(4));
        self.cpu.load_state(&CpuState {
            a: [0; 7],
            d: [0; 8],
            usp: 0,
            ssp,
            sr: 0x2700,
            pc: pc.wrapping_add(4),
            prefetch: [
                self.bus.ram.peek_word(pc),
                self.bus.ram.peek_word(pc.wrapping_add(2)),
            ],
        });
        self.cpu.clear_journal();
        self.instrs = 0;
        self.clocks = 0;
        self.status = "idle";
        self.trace.clear();
        self.buslog.clear();
        self.bus.wlog.clear();
        self.sst = None;
    }

    fn load_bytes(&mut self, name: &str, data: Vec<u8>) {
        self.bus.ram.clear_touched();
        for (i, c) in data.chunks(2).enumerate() {
            let w = u16::from_be_bytes([c[0], c.get(1).copied().unwrap_or(0)]);
            self.bus.ram.poke_word(2 * i as u32, w);
        }
        self.rom = Some((name.to_string(), data));
        self.reset_registers();
    }

    /// Loads a binary of the roms directory at address 0.
    pub fn load(&mut self, name: &str) -> Result<(), String> {
        if !safe_relative(name) {
            return Err(format!("invalid binary name `{name}`"));
        }
        let path = self.roms_dir.join(name);
        let data = std::fs::read(&path).map_err(|_| format!("unknown binary `{name}`"))?;
        self.load_bytes(name, data);
        Ok(())
    }

    /// The `.bin` files of the roms directory (relative path, size), sorted.
    pub fn list_roms(&self) -> Vec<(String, u64)> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, u64)>, depth: u32) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.filter_map(Result::ok) {
                let p = e.path();
                if p.is_dir() && depth < 4 {
                    walk(&p, base, out, depth + 1);
                } else if p.extension().is_some_and(|x| x == "bin") {
                    let rel = p.strip_prefix(base).unwrap_or(&p);
                    let name = rel.to_string_lossy().replace('\\', "/");
                    out.push((name, e.metadata().map_or(0, |m| m.len())));
                }
            }
        }
        let mut v = Vec::new();
        walk(&self.roms_dir, &self.roms_dir, &mut v, 0);
        v.sort();
        v
    }

    /// True while a `run` is in progress.
    pub fn is_running(&self) -> bool {
        self.status == "running"
    }

    /// Current speed index.
    pub fn speed(&self) -> u8 {
        self.speed
    }

    /// Updates the measured rates from the counters seen `dt` seconds ago.
    pub fn set_rates(&mut self, ips: f64, mhz: f64) {
        self.ips = ips;
        self.mhz = mhz;
    }

    /// (instructions, clocks) executed since the last load.
    pub fn counters(&self) -> (u64, u64) {
        (self.instrs, self.clocks)
    }

    fn pc_instr(&self) -> u32 {
        self.cpu.regs.pc.wrapping_sub(4)
    }

    fn words_at(&self, addr: u32) -> [u16; 5] {
        let mut w = [0u16; 5];
        for (i, x) in w.iter_mut().enumerate() {
            *x = self.bus.ram.peek_word(addr.wrapping_add(2 * i as u32));
        }
        w
    }

    fn text_at(&self, addr: u32) -> (String, usize) {
        let mut s = String::new();
        let n = disasm(&self.words_at(addr), addr, &mut s);
        (s, n)
    }

    fn absorb_journal(&mut self) {
        for e in self.cpu.journal() {
            let r = row(e);
            if let Some(s) = self.sst.as_mut() {
                s.actual.push(r);
            }
            self.buslog.push_front(r);
        }
        self.buslog.truncate(BUSLOG_LEN);
        self.cpu.clear_journal();
    }

    fn stopped_status(&self) -> &'static str {
        if self.cpu.regs.ir == 0x4E72 {
            "done"
        } else {
            "halted"
        }
    }

    /// Executes one instruction; false when nothing ran (STOP, illegal opcode, halted CPU).
    fn step_instr(&mut self) -> bool {
        let pc = self.pc_instr();
        let n = self.cpu.step(&mut self.bus);
        self.absorb_journal();
        if n == 0 {
            self.status = self.stopped_status();
            return false;
        }
        self.instrs += 1;
        self.clocks += u64::from(n);
        let text = self.text_at(pc).0;
        if self.trace.len() == TRACE_LEN {
            self.trace.pop_back();
        }
        self.trace.push_front((pc, text));
        true
    }

    /// Executes up to `n` instructions; with `stop_on_bp`, stops (status `bp`) when the next
    /// instruction is at a breakpoint.
    pub fn run_n(&mut self, n: u64, stop_on_bp: bool) {
        for _ in 0..n {
            if !self.step_instr() {
                return;
            }
            if stop_on_bp && self.bps.contains(&self.pc_instr()) {
                self.status = "bp";
                return;
            }
        }
    }

    /// Executes `n` clocks (an instruction starts when the previous one is complete).
    pub fn run_clocks(&mut self, n: u64) {
        for _ in 0..n {
            match self.cpu.step_clock(&mut self.bus) {
                None => {
                    self.status = self.stopped_status();
                    break;
                }
                Some(done) => {
                    self.clocks += 1;
                    if done {
                        self.instrs += 1;
                        self.absorb_journal();
                    }
                }
            }
        }
        self.absorb_journal();
    }

    fn follow_base(&self) -> u32 {
        let r = &self.cpu.regs;
        let base = match self.view.follow {
            Follow::Pc => self.pc_instr(),
            Follow::Sp => r.a7(),
            Follow::An(i) => r.a[i],
            Follow::Manual => self.view.mem_base,
        };
        base & 0x00FF_FFF0
    }

    /// Builds the snapshot; `take` drains the console and the write log (state sent to the page).
    pub fn snapshot(&mut self, take: bool) -> Snapshot {
        let r = self.cpu.regs;
        let mem_base = self.follow_base();
        self.view.mem_base = mem_base;
        let mut dis = Vec::new();
        let mut addr = self.pc_instr();
        for _ in 0..self.view.dis_rows {
            let (text, n) = self.text_at(addr);
            let words = self.words_at(addr)[..n].to_vec();
            dis.push(DisRow { addr, words, text });
            addr = addr.wrapping_add(2 * n as u32);
        }
        let sp = r.a7();
        let snap = Snapshot {
            status: self.status,
            instrs: self.instrs,
            clocks: self.clocks,
            ips: self.ips,
            mhz: self.mhz,
            regs: r,
            bus: self.buslog.front().copied(),
            buslog: self.buslog.iter().copied().collect(),
            dis,
            trace: self
                .trace
                .iter()
                .take(self.view.trace_rows)
                .cloned()
                .collect(),
            stack: (0..self.view.stack_rows)
                .map(|i| {
                    let a = sp.wrapping_add(2 * i as u32);
                    (a, self.bus.ram.peek_word(a))
                })
                .collect(),
            mem_base,
            mem: (0..256u32)
                .map(|i| self.bus.ram.peek_byte(mem_base.wrapping_add(i)))
                .collect(),
            wlog: self.bus.wlog.iter().copied().collect(),
            console_delta: if take {
                std::mem::take(&mut self.bus.console)
            } else {
                self.bus.console.clone()
            },
            bps: self.bps.clone(),
            rom: self.rom.as_ref().map(|(n, d)| (n.clone(), d.len())),
        };
        if take {
            self.bus.wlog.clear();
        }
        snap
    }

    fn state_msg(&mut self) -> String {
        self.snapshot(true).to_json()
    }

    fn roms_msg(&self) -> String {
        let mut o = String::from("{\"type\":\"roms\",\"roms\":[");
        for (i, (n, s)) in self.list_roms().iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(o, "{{\"name\":\"{}\",\"size\":{s}}}", esc(n));
        }
        o.push_str("]}");
        o
    }

    /// `sst_files` message: the stems of `dir` and the number of tests of each file.
    pub fn sst_files(dir: &Path) -> String {
        let mut files: Vec<(String, u32)> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter_map(|e| {
                        let p = e.path();
                        let stem = p
                            .file_name()?
                            .to_string_lossy()
                            .strip_suffix(".json.bin")?
                            .to_string();
                        let head = std::fs::read(&p)
                            .ok()
                            .map(|b| b.into_iter().take(8).collect::<Vec<u8>>())?;
                        let n = u32::from_le_bytes(head.get(4..8)?.try_into().ok()?);
                        Some((stem, n))
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        let mut o = String::from("{\"type\":\"sst_files\",\"files\":[");
        for (i, (s, n)) in files.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(o, "{{\"stem\":\"{}\",\"tests\":{n}}}", esc(s));
        }
        o.push_str("]}");
        o
    }

    fn sst_load(&mut self, dir: &Path, stem: &str, index: usize) -> Result<(), String> {
        if !safe_relative(stem) || stem.contains('/') || stem.contains('\\') {
            return Err(format!("invalid test file `{stem}`"));
        }
        let path = dir.join(format!("{stem}.json.bin"));
        let bytes = std::fs::read(&path).map_err(|_| format!("unknown test file `{stem}`"))?;
        let tests = ssc::parse_file(&bytes)?;
        let t = tests
            .get(index)
            .ok_or_else(|| format!("{stem}: test {index} out of range (0..{})", tests.len()))?;
        self.bus.ram.clear_touched();
        for &(addr, word) in &t.initial.ram {
            self.bus.ram.poke_word(addr, word);
        }
        let s = &t.initial;
        self.cpu.load_state(&CpuState {
            d: s.d,
            a: s.a,
            usp: s.usp,
            ssp: s.ssp,
            sr: s.sr as u16,
            pc: s.pc,
            prefetch: [s.prefetch[0] as u16, s.prefetch[1] as u16],
        });
        self.cpu.clear_journal();
        self.instrs = 0;
        self.clocks = 0;
        self.status = "idle";
        self.trace.clear();
        self.buslog.clear();
        self.rom = None;
        self.sst = Some(SstDebug {
            name: t.name.clone(),
            index,
            expected: t.trans.clone(),
            actual: Vec::new(),
        });
        Ok(())
    }

    /// `sst_test` message of the test being debugged.
    fn sst_msg(&self) -> Option<String> {
        let s = self.sst.as_ref()?;
        let mut o = String::new();
        let _ = write!(
            o,
            "{{\"type\":\"sst_test\",\"name\":\"{}\",\"index\":{},\"expected\":[",
            esc(&s.name),
            s.index
        );
        for (i, t) in s.expected.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let _ = write!(
                o,
                "{{\"kind\":\"{}\",\"cycles\":{},\"fc\":{},\"addr\":{},\"data\":{},\"uds\":{},\"lds\":{}}}",
                kind_str(t.kind),
                t.cycles,
                t.fc,
                t.addr,
                t.data,
                t.uds,
                t.lds
            );
        }
        o.push_str("],\"actual\":[");
        for (i, a) in s.actual.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            let k = match a.kind {
                0 => "n",
                1 => "w",
                3 => "t",
                4 => "re",
                5 => "we",
                _ => "r",
            };
            let _ = write!(
                o,
                "{{\"kind\":\"{k}\",\"cycles\":{},\"fc\":{},\"addr\":{},\"data\":{},\"uds\":{},\"lds\":{}}}",
                a.clocks,
                a.fc,
                a.addr,
                a.data,
                u8::from(a.uds),
                u8::from(a.lds)
            );
        }
        o.push_str("],\"diff\":[");
        let n = s.expected.len().max(s.actual.len());
        let mut first = true;
        for i in 0..n {
            let same = match (s.expected.get(i), s.actual.get(i)) {
                (Some(e), Some(a)) => {
                    kind_str(e.kind) == a.kind_char_ext()
                        && e.cycles == a.clocks
                        && (e.kind == Kind::Idle
                            || (e.fc == u32::from(a.fc)
                                && e.addr == a.addr
                                && e.data == u32::from(a.data)
                                && e.uds == u32::from(a.uds)
                                && e.lds == u32::from(a.lds)))
                }
                _ => false,
            };
            if !same {
                if !first {
                    o.push(',');
                }
                first = false;
                let _ = write!(o, "{i}");
            }
        }
        o.push_str("]}");
        Some(o)
    }

    /// Executes a command and returns the JSON messages to send, in order.
    pub fn handle(&mut self, cmd: Command, sst_dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        match cmd {
            Command::Hello => {
                out.push(format!(
                    "{{\"type\":\"hello\",\"version\":\"{}\"}}",
                    crate::VERSION
                ));
                out.push(self.roms_msg());
            }
            Command::ListRoms => {
                out.push(self.roms_msg());
                return out;
            }
            Command::Load { name } => {
                if let Err(e) = self.load(&name) {
                    return vec![error_json(&e)];
                }
            }
            Command::Upload { name, data } => {
                if data.is_empty() {
                    return vec![error_json("empty upload")];
                }
                let name = Path::new(&name).file_name().map_or_else(
                    || "upload.bin".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.load_bytes(&name, data);
            }
            Command::Run => {
                if self.status == "idle" || self.status == "bp" {
                    self.status = "running";
                }
            }
            Command::Pause => {
                if self.status == "running" {
                    self.status = "idle";
                }
            }
            Command::Step { n } => {
                if self.status == "bp" || self.status == "running" {
                    self.status = "idle";
                }
                self.run_n(n.clamp(1, 1_000_000), false);
            }
            Command::StepClock { n } => {
                if self.status == "bp" || self.status == "running" {
                    self.status = "idle";
                }
                self.run_clocks(n.clamp(1, 10_000_000));
            }
            Command::Reset => match self.rom.clone() {
                Some((name, data)) => self.load_bytes(&name, data),
                None => self.reset_registers(),
            },
            Command::Speed { idx } => {
                if usize::from(idx) >= SPEEDS.len() {
                    return vec![error_json(&format!(
                        "speed index {idx} out of range (0..7)"
                    ))];
                }
                self.speed = idx;
            }
            Command::BpAdd { addr } => {
                let a = addr & 0x00FF_FFFE;
                if !self.bps.contains(&a) {
                    self.bps.push(a);
                    self.bps.sort_unstable();
                }
            }
            Command::BpDel { addr } => {
                let a = addr & 0x00FF_FFFE;
                self.bps.retain(|&x| x != a);
            }
            Command::View {
                mem_base,
                dis_rows,
                trace_rows,
                stack_rows,
                follow,
            } => {
                if let Some(b) = mem_base {
                    self.view.mem_base = b;
                }
                if let Some(n) = dis_rows {
                    self.view.dis_rows = n.min(200);
                }
                if let Some(n) = trace_rows {
                    self.view.trace_rows = n.min(TRACE_LEN);
                }
                if let Some(n) = stack_rows {
                    self.view.stack_rows = n.min(200);
                }
                if let Some(f) = follow {
                    self.view.follow = match f.as_str() {
                        "pc" => Follow::Pc,
                        "sp" => Follow::Sp,
                        "man" => Follow::Manual,
                        s => match s.strip_prefix("an").and_then(|d| d.parse::<usize>().ok()) {
                            Some(i) if i < 7 => Follow::An(i),
                            _ => return vec![error_json(&format!("unknown follow `{f}`"))],
                        },
                    };
                }
            }
            Command::SstList => {
                return vec![Self::sst_files(sst_dir)];
            }
            Command::SstLoad { stem, index } => {
                if let Err(e) = self.sst_load(sst_dir, &stem, index) {
                    return vec![error_json(&e)];
                }
            }
            Command::ConsoleClear => self.bus.console.clear(),
        }
        out.push(self.state_msg());
        if let Some(m) = self.sst_msg() {
            out.push(m);
        }
        out
    }
}

impl BusRow {
    /// Like `kind_char` but distinguishes the address error kinds (`re`, `we`).
    fn kind_char_ext(&self) -> &'static str {
        match self.kind {
            4 => "re",
            5 => "we",
            _ => self.kind_char(),
        }
    }
}

/// A simulation thread driven by [`Command`]s.
pub struct Session {
    tx: Option<Sender<Command>>,
    out: Mutex<Receiver<String>>,
    latest: Arc<Mutex<Snapshot>>,
    handle: Option<JoinHandle<()>>,
}

impl Session {
    /// Starts the simulation thread; `roms_dir` holds the binaries, the SingleStepTests files are
    /// read from `tests/singlestep`.
    pub fn spawn(roms_dir: PathBuf) -> Session {
        Self::spawn_with(roms_dir, PathBuf::from("tests/singlestep"))
    }

    /// Like [`Session::spawn`] with an explicit SingleStepTests directory.
    pub fn spawn_with(roms_dir: PathBuf, sst_dir: PathBuf) -> Session {
        let (tx, rx) = mpsc::channel::<Command>();
        let (otx, orx) = mpsc::channel::<String>();
        let mut machine = Machine::new(roms_dir);
        let latest = Arc::new(Mutex::new(machine.snapshot(false)));
        let l2 = Arc::clone(&latest);
        let handle =
            std::thread::spawn(move || thread_main(&mut machine, &rx, &otx, &l2, &sst_dir));
        Session {
            tx: Some(tx),
            out: Mutex::new(orx),
            latest,
            handle: Some(handle),
        }
    }

    /// Sends a command to the simulation thread.
    pub fn send(&self, cmd: Command) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(cmd);
        }
    }

    /// The last state published by the thread.
    pub fn latest_state(&self) -> Snapshot {
        self.latest.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Next outgoing JSON message, waiting up to `d`.
    pub fn recv_timeout(&self, d: Duration) -> Option<String> {
        self.out.lock().ok()?.recv_timeout(d).ok()
    }

    /// Stops the thread and waits for it.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.tx = None;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn thread_main(
    m: &mut Machine,
    rx: &Receiver<Command>,
    tx: &Sender<String>,
    latest: &Arc<Mutex<Snapshot>>,
    sst_dir: &Path,
) {
    let mut last_push = Instant::now();
    let mut last_tick = Instant::now();
    let mut acc = 0.0f64;
    let mut rate_t = Instant::now();
    let mut rate_base = m.counters();
    loop {
        let cmd = if m.is_running() {
            match rx.try_recv() {
                Ok(c) => Some(c),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => break,
            }
        };
        if let Some(c) = cmd {
            for msg in m.handle(c, sst_dir) {
                let _ = tx.send(msg);
            }
            if let Ok(mut l) = latest.lock() {
                *l = m.snapshot(false);
            }
            acc = 0.0;
            last_tick = Instant::now();
            last_push = last_tick;
            continue;
        }
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;
        let ips = SPEEDS[usize::from(m.speed())];
        let n = if ips == 0 {
            20_000
        } else {
            acc += f64::from(ips) * dt;
            let k = acc as u64;
            acc -= k as f64;
            k
        };
        if n > 0 {
            m.run_n(n, true);
        }
        let el = now.duration_since(rate_t).as_secs_f64();
        if el >= 0.5 {
            let (i, c) = m.counters();
            m.set_rates(
                (i - rate_base.0) as f64 / el,
                (c - rate_base.1) as f64 / el / 1e6,
            );
            rate_base = (i, c);
            rate_t = now;
        }
        if !m.is_running() || now.duration_since(last_push) >= STATE_INTERVAL {
            let s = m.snapshot(true);
            let _ = tx.send(s.to_json());
            if let Ok(mut l) = latest.lock() {
                *l = s;
            }
            last_push = now;
        }
        if ips != 0 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("m68k_session_{tag}_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    /// Image: SSP 0x1000, PC 0x100, `code` words at 0x100.
    fn image(code: &[u16]) -> Vec<u8> {
        let mut v = vec![0u8; 0x100];
        v[2] = 0x10;
        v[6] = 0x01;
        for w in code {
            v.extend_from_slice(&w.to_be_bytes());
        }
        v
    }

    fn machine(code: &[u16]) -> Machine {
        let mut m = Machine::new(tmp_dir("m"));
        m.load_bytes("t.bin", image(code));
        m
    }

    fn state(m: &mut Machine, c: Command) -> Snapshot {
        m.handle(c, Path::new("none"));
        m.snapshot(false)
    }

    #[test]
    fn e13_01_step() {
        let mut m = machine(&[0x4E71, 0x4E71, 0x4E71]);
        let s = state(&mut m, Command::Step { n: 1 });
        assert_eq!(
            (s.regs.pc.wrapping_sub(4), s.instrs, s.clocks),
            (0x102, 1, 4)
        );
        assert_eq!(s.status, "idle");
        let s = state(&mut m, Command::Step { n: 2 });
        assert_eq!((s.regs.pc.wrapping_sub(4), s.instrs), (0x106, 3));
        assert_eq!(s.trace.len(), 3);
        assert_eq!(s.trace[0].0, 0x104);
    }

    #[test]
    fn e13_01_step_clock() {
        let mut m = machine(&[0x4E71, 0x4E71]);
        let s = state(&mut m, Command::StepClock { n: 4 });
        assert_eq!((s.clocks, s.instrs), (4, 1));
        assert_eq!(s.regs.pc.wrapping_sub(4), 0x102);
        let s = state(&mut m, Command::StepClock { n: 2 });
        assert_eq!((s.clocks, s.instrs), (6, 1));
        assert_eq!(s.buslog.len(), 1);
    }

    #[test]
    fn e13_01_breakpoint() {
        let mut m = machine(&[0x4E71; 8]);
        state(&mut m, Command::BpAdd { addr: 0x108 });
        state(&mut m, Command::Run);
        assert!(m.is_running());
        m.run_n(1000, true);
        let s = m.snapshot(false);
        assert_eq!(
            (s.status, s.regs.pc.wrapping_sub(4), s.instrs),
            ("bp", 0x108, 4)
        );
        assert_eq!(s.bps, [0x108]);
        // a breakpoint at the current PC does not stop the next run
        state(&mut m, Command::Run);
        m.run_n(1, true);
        assert_eq!(m.snapshot(false).regs.pc.wrapping_sub(4), 0x10A);
        state(&mut m, Command::BpDel { addr: 0x108 });
        assert!(m.snapshot(false).bps.is_empty());
    }

    #[test]
    fn e13_01_pause() {
        let dir = tmp_dir("pause");
        std::fs::write(dir.join("nop.bin"), image(&[0x4E71; 0x400])).unwrap();
        let s = Session::spawn_with(dir, PathBuf::from("none"));
        s.send(Command::Load {
            name: "nop.bin".into(),
        });
        s.send(Command::Speed { idx: 7 });
        s.send(Command::Run);
        std::thread::sleep(Duration::from_millis(100));
        s.send(Command::Pause);
        // wait for the idle state
        let mut idle = false;
        for _ in 0..200 {
            if s.latest_state().status == "idle" && s.latest_state().instrs > 0 {
                idle = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(idle, "status {}", s.latest_state().status);
        let a = s.latest_state().instrs;
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(s.latest_state().instrs, a);
        s.stop();
    }

    #[test]
    fn e13_01_reset() {
        let mut m = machine(&[0x4E71, 0x7005]);
        state(&mut m, Command::Step { n: 2 });
        let s = state(&mut m, Command::Reset);
        assert_eq!(
            (s.regs.ssp, s.regs.pc.wrapping_sub(4), s.instrs, s.regs.d[0]),
            (0x1000, 0x100, 0, 0)
        );
        assert_eq!(s.rom, Some(("t.bin".to_string(), 0x104)));
    }

    #[test]
    fn e13_01_speed_index() {
        let mut m = machine(&[0x4E71]);
        let r = m.handle(Command::Speed { idx: 8 }, Path::new("none"));
        assert!(r[0].contains("\"type\":\"error\""), "{r:?}");
        assert_eq!(m.speed(), 5);
        let r = m.handle(Command::Speed { idx: 7 }, Path::new("none"));
        assert!(r[0].contains("\"type\":\"state\""));
        assert_eq!(m.speed(), 7);
    }

    #[test]
    fn e13_01_console_and_halt() {
        // move.b #$41,$ff00.w ; (0xA000 = line A: stops)
        let mut m = machine(&[0x11FC, 0x0041, 0xFF00, 0xA000]);
        m.run_n(10, false);
        let s = m.snapshot(true);
        assert_eq!(s.console_delta, "A");
        assert_eq!(s.status, "halted");
        assert!(s.wlog.contains(&CONSOLE_ADDR));
        assert_eq!(m.snapshot(true).console_delta, "");
    }

    #[test]
    fn e13_01_view_and_dis() {
        let mut m = machine(&[0x7001, 0x4E71, 0x4E75]);
        let r = m.handle(
            Command::View {
                mem_base: None,
                dis_rows: Some(3),
                trace_rows: None,
                stack_rows: Some(2),
                follow: Some("sp".into()),
            },
            Path::new("none"),
        );
        assert!(r[0].contains("moveq #1,d0"), "{}", r[0]);
        let s = m.snapshot(false);
        assert_eq!((s.dis.len(), s.stack.len(), s.mem_base), (3, 2, 0x1000));
        let r = m.handle(
            Command::View {
                mem_base: None,
                dis_rows: None,
                trace_rows: None,
                stack_rows: None,
                follow: Some("bogus".into()),
            },
            Path::new("none"),
        );
        assert!(r[0].contains("error"));
    }
}
