//! End-to-end: the `m68k-web` binary, a WebSocket client, a small program.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

type Client = WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Image: SSP 0x1000, PC 0x100, then `code` at 0x100.
fn image(code: &[u16]) -> Vec<u8> {
    let mut v = vec![0u8; 0x100];
    v[2] = 0x10;
    v[6] = 0x01;
    for w in code {
        v.extend_from_slice(&w.to_be_bytes());
    }
    v
}

fn start(tag: &str) -> Server {
    let dir = std::env::temp_dir().join(format!("m68k_e2e_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // nop ; moveq #1,d0 ; addq.l #1,d0 ; bra.s *
    std::fs::write(
        dir.join("prog.bin"),
        image(&[0x4E71, 0x7001, 0x5280, 0x60FE]),
    )
    .unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = Command::new(env!("CARGO_BIN_EXE_m68k-web"))
        .args(["--port", &port.to_string(), "--roms"])
        .arg(&dir)
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("m68k-web starts");
    Server { child, port }
}

fn connect(s: &Server) -> Client {
    let t0 = Instant::now();
    loop {
        if TcpStream::connect(("127.0.0.1", s.port)).is_ok() {
            if let Ok((c, _)) = tungstenite::connect(format!("ws://127.0.0.1:{}/ws", s.port)) {
                return c;
            }
        }
        assert!(
            t0.elapsed() < Duration::from_secs(10),
            "server not reachable"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn say(c: &mut Client, json: &str) {
    c.send(Message::text(json)).unwrap();
}

fn next(c: &mut Client) -> String {
    loop {
        if let Message::Text(t) = c.read().unwrap() {
            return t.as_str().to_string();
        }
    }
}

/// Next message of type `want`.
fn until(c: &mut Client, want: &str) -> String {
    let pat = format!("\"type\":\"{want}\"");
    loop {
        let m = next(c);
        if m.contains(&pat) {
            return m;
        }
    }
}

/// First integer after `key` (e.g. `"clocks":`).
fn int(m: &str, key: &str) -> u64 {
    let at = m.find(key).unwrap_or_else(|| panic!("{key} in {m}")) + key.len();
    let digits: String = m[at..].chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

#[test]
fn e13_07_hello_state() {
    let s = start("hello");
    let mut c = connect(&s);
    say(&mut c, "{\"cmd\":\"hello\"}");
    assert!(next(&mut c).contains("\"type\":\"hello\""));
    let roms = next(&mut c);
    assert!(
        roms.contains("\"type\":\"roms\"") && roms.contains("prog.bin"),
        "{roms}"
    );
    let st = next(&mut c);
    assert!(st.contains("\"type\":\"state\""), "{st}");
    for k in [
        "\"regs\":{\"d\":[",
        "\"dis\":[",
        "\"mem\":{\"base\":",
        "\"status\":\"idle\"",
    ] {
        assert!(st.contains(k), "{k} in {st}");
    }
    assert!(st.starts_with('{') && st.ends_with('}'));
}

#[test]
fn e13_07_load_step() {
    let s = start("load");
    let mut c = connect(&s);
    say(&mut c, "{\"cmd\":\"load\",\"name\":\"prog.bin\"}");
    let st = until(&mut c, "state");
    assert_eq!(int(&st, "\"pc\":"), 0x100);
    assert_eq!(int(&st, "\"ssp\":"), 0x1000);
    for expect in [0x102, 0x104, 0x106] {
        say(&mut c, "{\"cmd\":\"step\"}");
        let st = until(&mut c, "state");
        assert_eq!(int(&st, "\"pc\":"), expect, "{st}");
    }
}

#[test]
fn e13_07_clocks() {
    let s = start("clocks");
    let mut c = connect(&s);
    say(&mut c, "{\"cmd\":\"load\",\"name\":\"prog.bin\"}");
    until(&mut c, "state");
    say(&mut c, "{\"cmd\":\"step\",\"n\":3}");
    let st = until(&mut c, "state");
    // nop 4 + moveq 4 + addq.l #1,d0 8
    assert_eq!(int(&st, "\"clocks\":"), 16, "{st}");
    assert_eq!(int(&st, "\"instrs\":"), 3);
    assert_eq!(int(&st, "\"d\":["), 2);
    // clock stepping reaches the same total
    say(&mut c, "{\"cmd\":\"reset\"}");
    until(&mut c, "state");
    say(&mut c, "{\"cmd\":\"step_clock\",\"n\":16}");
    let st = until(&mut c, "state");
    assert_eq!(
        (int(&st, "\"clocks\":"), int(&st, "\"instrs\":")),
        (16, 3),
        "{st}"
    );
    assert_eq!(int(&st, "\"pc\":"), 0x106);
}

#[test]
fn e13_07_sst_roundtrip() {
    if !repo_root().join("tests/singlestep/NOP.json.bin").exists() {
        return;
    }
    let s = start("sst");
    let mut c = connect(&s);
    say(&mut c, "{\"cmd\":\"sst_list\"}");
    let m = until(&mut c, "sst_files");
    assert!(m.contains("{\"stem\":\"NOP\",\"tests\":2500}"), "{m}");
    say(
        &mut c,
        "{\"cmd\":\"sst_load\",\"stem\":\"NOP\",\"index\":0}",
    );
    let m = until(&mut c, "sst_test");
    assert!(m.contains("\"actual\":[]"), "{m}");
    say(&mut c, "{\"cmd\":\"step\"}");
    let m = until(&mut c, "sst_test");
    assert!(
        m.contains("\"diff\":[]") && !m.contains("\"actual\":[]"),
        "{m}"
    );
}
