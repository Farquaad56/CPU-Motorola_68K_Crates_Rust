//! WebSocket endpoint `/ws`: client JSON messages in, server JSON messages out.
//!
//! One simulation [`Session`] per connection. The connection thread alternates between reading
//! (short read timeout) and forwarding the messages the session produces.

use crate::http::{Head, WsHandler};
use base64::Engine;
use puce_m68k_machines::session::{Command, Session};
use puce_m68k_machines::snapshot::esc;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::protocol::Role;
use tungstenite::{Message, WebSocket};

/// A JSON value (flat objects only).
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// String.
    Str(String),
    /// Number.
    Num(f64),
    /// Boolean.
    Bool(bool),
    /// `null`.
    Null,
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at {}", char::from(c), self.i))
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(|t| u32::from_str_radix(t, 16).ok())
            .ok_or("bad \\u escape")?;
        self.i += 4;
        Ok(h)
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("unterminated string")?;
                    self.i += 1;
                    let ch = match e {
                        b'"' | b'\\' | b'/' => char::from(e),
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let mut u = self.hex4()?;
                            if (0xD800..0xDC00).contains(&u)
                                && self.s.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                u = 0x10000
                                    + ((u - 0xD800) << 10)
                                    + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            char::from_u32(u).unwrap_or('\u{FFFD}')
                        }
                        _ => return Err("bad escape".into()),
                    };
                    let mut b = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                }
                c if c < 0x20 => return Err("control character in string".into()),
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| "invalid utf-8".to_string())
    }

    fn value(&mut self) -> Result<Val, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'"') => self.string().map(Val::Str),
            Some(b't') if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Val::Bool(true))
            }
            Some(b'f') if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Val::Bool(false))
            }
            Some(b'n') if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(Val::Null)
            }
            Some(c) if c.is_ascii_digit() || *c == b'-' => {
                let st = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[st..self.i])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(Val::Num)
                    .ok_or_else(|| "bad number".to_string())
            }
            Some(b'{' | b'[') => Err("nested values are not supported".into()),
            _ => Err(format!("unexpected input at {}", self.i)),
        }
    }
}

/// Parses a flat JSON object into its (key, value) pairs.
pub fn parse_object(text: &str) -> Result<Vec<(String, Val)>, String> {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
    };
    p.eat(b'{')?;
    let mut out = Vec::new();
    p.ws();
    if p.s.get(p.i) == Some(&b'}') {
        p.i += 1;
    } else {
        loop {
            let k = p.string()?;
            p.eat(b':')?;
            out.push((k, p.value()?));
            p.ws();
            match p.s.get(p.i) {
                Some(b',') => p.i += 1,
                Some(b'}') => {
                    p.i += 1;
                    break;
                }
                _ => return Err(format!("expected `,` or `}}` at {}", p.i)),
            }
        }
    }
    p.ws();
    if p.i != p.s.len() {
        return Err("trailing characters".into());
    }
    Ok(out)
}

fn get<'a>(o: &'a [(String, Val)], k: &str) -> Option<&'a Val> {
    o.iter().find(|(n, _)| n == k).map(|(_, v)| v)
}

fn num(o: &[(String, Val)], k: &str) -> Result<Option<u64>, String> {
    match get(o, k) {
        None | Some(Val::Null) => Ok(None),
        Some(Val::Num(n)) if *n >= 0.0 && n.fract() == 0.0 && *n < 1.8e19 => Ok(Some(*n as u64)),
        Some(Val::Str(s)) => {
            let t = s.trim();
            let r = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix('$')) {
                u64::from_str_radix(h, 16)
            } else {
                t.parse::<u64>()
            };
            r.map(Some).map_err(|_| format!("bad number in `{k}`"))
        }
        Some(_) => Err(format!("bad number in `{k}`")),
    }
}

fn text(o: &[(String, Val)], k: &str) -> Result<String, String> {
    match get(o, k) {
        Some(Val::Str(s)) => Ok(s.clone()),
        _ => Err(format!("missing string field `{k}`")),
    }
}

/// Command of a client message (a JSON object with a `cmd` field).
pub fn parse_command(msg: &str) -> Result<Command, String> {
    let o = parse_object(msg)?;
    let cmd = text(&o, "cmd")?;
    let addr = |k: &str| -> Result<u32, String> {
        num(&o, k)?
            .map(|v| v as u32)
            .ok_or_else(|| format!("missing field `{k}`"))
    };
    let rows = |k: &str| -> Result<Option<usize>, String> { Ok(num(&o, k)?.map(|v| v as usize)) };
    Ok(match cmd.as_str() {
        "hello" => Command::Hello,
        "list_roms" => Command::ListRoms,
        "load" => Command::Load {
            name: text(&o, "name")?,
        },
        "upload" => Command::Upload {
            name: text(&o, "name")?,
            data: base64::engine::general_purpose::STANDARD
                .decode(text(&o, "data_b64")?)
                .map_err(|e| format!("bad base64: {e}"))?,
        },
        "run" => Command::Run,
        "pause" => Command::Pause,
        "step" => Command::Step {
            n: num(&o, "n")?.unwrap_or(1),
        },
        "step_clock" => Command::StepClock {
            n: num(&o, "n")?.unwrap_or(1),
        },
        "reset" => Command::Reset,
        "speed" => Command::Speed {
            idx: u8::try_from(num(&o, "idx")?.ok_or("missing field `idx`")?)
                .map_err(|_| "speed index out of range".to_string())?,
        },
        "bp_add" => Command::BpAdd {
            addr: addr("addr")?,
        },
        "bp_del" => Command::BpDel {
            addr: addr("addr")?,
        },
        "view" => Command::View {
            mem_base: num(&o, "mem_base")?.map(|v| v as u32),
            dis_rows: rows("dis_rows")?,
            trace_rows: rows("trace_rows")?,
            stack_rows: rows("stack_rows")?,
            follow: match get(&o, "follow") {
                Some(Val::Str(s)) => Some(s.clone()),
                _ => None,
            },
        },
        "sst_list" => Command::SstList,
        "sst_load" => Command::SstLoad {
            stem: text(&o, "stem")?,
            index: num(&o, "index")?.unwrap_or(0) as usize,
        },
        "console_clear" => Command::ConsoleClear,
        other => return Err(format!("unknown cmd `{other}`")),
    })
}

/// `error` message.
pub fn error_msg(m: &str) -> String {
    format!("{{\"type\":\"error\",\"message\":\"{}\"}}", esc(m))
}

/// Serves one upgraded connection until the client leaves or `stop` is set.
pub fn run_connection(mut stream: TcpStream, head: &Head, session: &Session, stop: &AtomicBool) {
    let Some(key) = head.header("sec-websocket-key") else {
        return;
    };
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let reply = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    if std::io::Write::write_all(&mut stream, reply.as_bytes()).is_err() {
        return;
    }
    let _ = stream.set_nodelay(true);
    if stream
        .set_read_timeout(Some(Duration::from_millis(10)))
        .is_err()
    {
        return;
    }
    let mut ws = WebSocket::from_raw_socket(stream, Role::Server, None);
    while !stop.load(Ordering::Relaxed) {
        match ws.read() {
            Ok(Message::Text(t)) => match parse_command(t.as_str()) {
                Ok(c) => session.send(c),
                Err(e) => {
                    if ws.send(Message::text(error_msg(&e))).is_err() {
                        break;
                    }
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => break,
        }
        while let Some(m) = session.recv_timeout(Duration::ZERO) {
            if ws.send(Message::text(m)).is_err() {
                return;
            }
        }
        let _ = ws.flush();
    }
    let _ = ws.close(None);
    let _ = ws.flush();
}

/// Handler for [`crate::http::serve`]: one session per connection.
pub fn handler(roms: PathBuf, sst: PathBuf, stop: Arc<AtomicBool>) -> Arc<WsHandler> {
    Arc::new(move |stream, head| {
        let session = Session::spawn_with(roms.clone(), sst.clone());
        run_connection(stream, &head, &session, &stop);
        session.stop();
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http;
    use std::net::SocketAddr;
    use std::time::Instant;

    type Client = WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

    struct Srv {
        addr: SocketAddr,
        stop: Arc<AtomicBool>,
        h: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for Srv {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.h.take() {
                let _ = h.join();
            }
        }
    }

    fn rom_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("m68k_ws_{tag}_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        // SSP 0x1000, PC 0x100, `bra.s *` at 0x100
        let mut v = vec![0u8; 0x100];
        v[2] = 0x10;
        v[6] = 0x01;
        v.extend_from_slice(&[0x60, 0xFE]);
        std::fs::write(d.join("loop.bin"), v).unwrap();
        d
    }

    fn start(tag: &str) -> Srv {
        start_sst(tag, PathBuf::from("none"))
    }

    fn start_sst(tag: &str, sst: PathBuf) -> Srv {
        let l = http::bind(0).unwrap();
        let addr = http::local_addr(&l).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let ws = handler(rom_dir(tag), sst, Arc::clone(&stop));
        let st2 = Arc::clone(&stop);
        let h = std::thread::spawn(move || http::serve(&l, &st2, &ws));
        Srv {
            addr,
            stop,
            h: Some(h),
        }
    }

    fn connect(s: &Srv) -> Client {
        let (c, _) = tungstenite::connect(format!("ws://{}/ws", s.addr)).unwrap();
        c
    }

    fn ty(m: &str) -> String {
        let at = m.find("\"type\":\"").unwrap() + 8;
        m[at..at + m[at..].find('"').unwrap()].to_string()
    }

    /// Next text message (waiting up to 5 s).
    fn next(c: &mut Client) -> String {
        loop {
            match c.read().unwrap() {
                Message::Text(t) => return t.as_str().to_string(),
                Message::Close(_) => panic!("closed"),
                _ => {}
            }
        }
    }

    #[test]
    fn e13_04_hello() {
        let s = start("hello");
        let mut c = connect(&s);
        c.send(Message::text("{\"cmd\":\"hello\"}")).unwrap();
        let types: Vec<String> = (0..3).map(|_| ty(&next(&mut c))).collect();
        assert_eq!(types, ["hello", "roms", "state"]);
    }

    #[test]
    fn e13_04_unknown_cmd() {
        let s = start("unk");
        let mut c = connect(&s);
        c.send(Message::text("{\"cmd\":\"nope\"}")).unwrap();
        let m = next(&mut c);
        assert_eq!(ty(&m), "error");
        assert!(m.contains("unknown cmd"), "{m}");
        // the connection is still usable
        c.send(Message::text("{\"cmd\":\"step\"}")).unwrap();
        assert_eq!(ty(&next(&mut c)), "state");
    }

    #[test]
    fn e13_04_bad_json() {
        let s = start("bad");
        let mut c = connect(&s);
        for bad in [
            "{not json",
            "[1]",
            "{\"cmd\":",
            "{\"cmd\":\"step\"} x",
            "{\"cmd\":{}}",
            "",
            "{\"cmd\":\"speed\",\"idx\":99}",
        ] {
            c.send(Message::text(bad)).unwrap();
            assert_eq!(ty(&next(&mut c)), "error", "{bad}");
        }
        c.send(Message::text("{\"cmd\":\"reset\"}")).unwrap();
        assert_eq!(ty(&next(&mut c)), "state");
    }

    #[test]
    fn e13_04_rate_limit() {
        let s = start("rate");
        let mut c = connect(&s);
        for m in [
            "{\"cmd\":\"load\",\"name\":\"loop.bin\"}",
            "{\"cmd\":\"speed\",\"idx\":7}",
            "{\"cmd\":\"run\"}",
        ] {
            c.send(Message::text(m)).unwrap();
        }
        // skip the immediate answers, then count the states in one second
        for _ in 0..3 {
            next(&mut c);
        }
        let t0 = Instant::now();
        let mut n = 0u32;
        while t0.elapsed() < Duration::from_secs(1) {
            if ty(&next(&mut c)) == "state" {
                n += 1;
            }
        }
        assert!((10..=31).contains(&n), "{n} states in 1 s");
        c.send(Message::text("{\"cmd\":\"pause\"}")).unwrap();
    }

    #[test]
    fn e13_04_parse() {
        let o = parse_object(" {\"a\": \"x\\u00e9\\n\", \"n\": -1.5e1, \"b\": true, \"z\": null} ")
            .unwrap();
        assert_eq!(get(&o, "a"), Some(&Val::Str("x\u{e9}\n".into())));
        assert_eq!(get(&o, "n"), Some(&Val::Num(-15.0)));
        assert_eq!(
            parse_command("{\"cmd\":\"step\",\"n\":5}"),
            Ok(Command::Step { n: 5 })
        );
        assert_eq!(
            parse_command("{\"cmd\":\"bp_add\",\"addr\":\"0x1F0\"}"),
            Ok(Command::BpAdd { addr: 0x1F0 })
        );
        assert_eq!(
            parse_command("{\"cmd\":\"upload\",\"name\":\"a.bin\",\"data_b64\":\"AQID\"}"),
            Ok(Command::Upload {
                name: "a.bin".into(),
                data: vec![1, 2, 3]
            })
        );
        assert!(parse_command("{\"cmd\":\"upload\",\"name\":\"a\",\"data_b64\":\"!!\"}").is_err());
    }

    /// Messages until one of type `want` (the others are dropped).
    fn until(c: &mut Client, want: &str) -> String {
        loop {
            let m = next(c);
            if ty(&m) == want {
                return m;
            }
        }
    }

    #[test]
    fn e13_05_list_roms() {
        let s = start("list");
        let mut c = connect(&s);
        c.send(Message::text("{\"cmd\":\"list_roms\"}")).unwrap();
        let m = next(&mut c);
        assert_eq!(ty(&m), "roms");
        assert!(m.contains("{\"name\":\"loop.bin\",\"size\":258}"), "{m}");
    }

    #[test]
    fn e13_05_load_unknown() {
        let s = start("unk_rom");
        let mut c = connect(&s);
        for name in ["nope.bin", "../x.bin", "/etc/passwd"] {
            c.send(Message::text(format!(
                "{{\"cmd\":\"load\",\"name\":\"{name}\"}}"
            )))
            .unwrap();
            let m = next(&mut c);
            assert_eq!(ty(&m), "error", "{name}: {m}");
        }
        c.send(Message::text("{\"cmd\":\"load\",\"name\":\"loop.bin\"}"))
            .unwrap();
        let m = next(&mut c);
        assert_eq!(ty(&m), "state");
        assert!(
            m.contains("\"rom\":{\"name\":\"loop.bin\",\"size\":258}"),
            "{m}"
        );
    }

    #[test]
    fn e13_05_upload_b64() {
        use base64::Engine;
        let s = start("upload");
        let mut c = connect(&s);
        // SSP 0x2000, PC 0x100, `moveq #7,d3` at 0x100
        let mut img = vec![0u8; 0x100];
        img[2] = 0x20;
        img[6] = 0x01;
        img.extend_from_slice(&[0x76, 0x07]);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&img);
        c.send(Message::text(format!(
            "{{\"cmd\":\"upload\",\"name\":\"dir/up.bin\",\"data_b64\":\"{b64}\"}}"
        )))
        .unwrap();
        let m = next(&mut c);
        assert_eq!(ty(&m), "state");
        assert!(
            m.contains("\"rom\":{\"name\":\"up.bin\",\"size\":258}"),
            "{m}"
        );
        c.send(Message::text("{\"cmd\":\"step\"}")).unwrap();
        let m = next(&mut c);
        assert!(m.contains("\"d\":[0,0,0,7,"), "{m}");
    }

    #[test]
    fn e13_05_sst_list() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/singlestep");
        if !std::path::Path::new(dir).join("NOP.json.bin").exists() {
            return;
        }
        let s = start_sst("sstl", PathBuf::from(dir));
        let mut c = connect(&s);
        c.send(Message::text("{\"cmd\":\"sst_list\"}")).unwrap();
        let m = next(&mut c);
        assert_eq!(ty(&m), "sst_files");
        assert!(m.contains("{\"stem\":\"NOP\",\"tests\":2500}"), "{m}");
    }

    #[test]
    fn e13_05_sst_load_diff() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/singlestep");
        if !std::path::Path::new(dir).join("NOP.json.bin").exists() {
            return;
        }
        let s = start_sst("sstd", PathBuf::from(dir));
        let mut c = connect(&s);
        c.send(Message::text(
            "{\"cmd\":\"sst_load\",\"stem\":\"NOP\",\"index\":3}",
        ))
        .unwrap();
        let m = until(&mut c, "sst_test");
        assert!(m.contains("NOP 4e71"), "{m}");
        // nothing executed yet: every expected transaction differs
        assert!(
            m.contains("\"actual\":[]") && m.contains("\"diff\":[0]"),
            "{m}"
        );
        c.send(Message::text("{\"cmd\":\"step\"}")).unwrap();
        let m = until(&mut c, "sst_test");
        assert!(m.contains("\"diff\":[]"), "{m}");
        assert!(!m.contains("\"actual\":[]"), "{m}");
        c.send(Message::text(
            "{\"cmd\":\"sst_load\",\"stem\":\"NOP\",\"index\":99999}",
        ))
        .unwrap();
        assert_eq!(ty(&next(&mut c)), "error");
        c.send(Message::text(
            "{\"cmd\":\"sst_load\",\"stem\":\"../NOP\",\"index\":0}",
        ))
        .unwrap();
        assert_eq!(ty(&next(&mut c)), "error");
    }
}
