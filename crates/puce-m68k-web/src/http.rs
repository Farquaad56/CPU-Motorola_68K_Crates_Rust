//! Minimal HTTP server on `std::net`: the embedded page, `/health`, 404 for the rest, and the
//! hand-over of `/ws` upgrades to the WebSocket handler. 127.0.0.1 only.
//!
//! The connection is handed over as a plain `TcpStream` (read timeout, `try_clone`), which an
//! upgraded `tiny_http` request does not allow: see D-045.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// The page served on `GET /`.
pub const INDEX: &str = include_str!("../../../docs/frontend/index.html");

/// Longest request head accepted.
const MAX_HEAD: usize = 16 * 1024;

/// Status, content type and body of an answer.
#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    /// HTTP status code.
    pub status: u16,
    /// `Content-Type` value.
    pub ctype: &'static str,
    /// Body text.
    pub body: &'static str,
}

/// Answer for `method` and `url` (query string and fragment ignored).
pub fn route(method: &str, url: &str) -> Reply {
    let path = url.split(['?', '#']).next().unwrap_or("");
    match (method, path) {
        ("GET" | "HEAD", "/") => Reply {
            status: 200,
            ctype: "text/html; charset=utf-8",
            body: INDEX,
        },
        ("GET" | "HEAD", "/health") => Reply {
            status: 200,
            ctype: "text/plain; charset=utf-8",
            body: "ok",
        },
        _ => Reply {
            status: 404,
            ctype: "text/plain; charset=utf-8",
            body: "not found",
        },
    }
}

/// Request line and headers.
#[derive(Clone, Debug, Default)]
pub struct Head {
    /// `GET`, `POST`...
    pub method: String,
    /// Target as sent (with the query string).
    pub url: String,
    /// Headers in order (names as sent).
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// Value of a header (name compared without case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Path of the target, without query string or fragment.
    pub fn path(&self) -> &str {
        self.url.split(['?', '#']).next().unwrap_or("")
    }

    /// True for `GET /ws` with `Upgrade: websocket`.
    pub fn is_ws_upgrade(&self) -> bool {
        self.method == "GET"
            && self.path() == "/ws"
            && self
                .header("upgrade")
                .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
    }
}

/// Parses a request head (up to the blank line, excluded).
pub fn parse_head(text: &str) -> Option<Head> {
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let url = first.next()?.to_string();
    if method.is_empty() || url.is_empty() || !first.next()?.starts_with("HTTP/") {
        return None;
    }
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    Some(Head {
        method,
        url,
        headers,
    })
}

/// Reads the head of a request; `None` on timeout, error or oversized head.
///
/// Bytes after the blank line are not consumed beyond the chunk read: the clients of this server
/// (browser, WebSocket handshake) send nothing before the answer.
pub fn read_head(s: &mut TcpStream) -> Option<Head> {
    s.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return parse_head(&String::from_utf8_lossy(&buf[..i]));
        }
        if buf.len() > MAX_HEAD {
            return None;
        }
    }
}

fn write_reply(s: &mut TcpStream, method: &str, r: &Reply) {
    let reason = if r.status == 200 { "OK" } else { "Not Found" };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        r.status,
        r.ctype,
        r.body.len()
    );
    let _ = s.write_all(head.as_bytes());
    if method != "HEAD" {
        let _ = s.write_all(r.body.as_bytes());
    }
    let _ = s.flush();
    let _ = s.shutdown(Shutdown::Write);
    // lingering close: wait for the client to close, so that unread data does not turn the
    // close into a reset that would cut the answer short
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    let mut sink = [0u8; 256];
    while s.read(&mut sink).is_ok_and(|n| n > 0) {}
}

/// Handler of the `/ws` upgrades: receives the connection and the request head.
pub type WsHandler = dyn Fn(TcpStream, Head) + Send + Sync;

fn connection(mut s: TcpStream, ws: &WsHandler) {
    let Some(head) = read_head(&mut s) else {
        return;
    };
    if head.is_ws_upgrade() {
        ws(s, head);
    } else {
        write_reply(&mut s, &head.method, &route(&head.method, &head.url));
    }
}

/// Binds 127.0.0.1:`port` (0 = any free port).
pub fn bind(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())
}

/// Address the listener is bound to.
pub fn local_addr(l: &TcpListener) -> Option<SocketAddr> {
    l.local_addr().ok()
}

/// Accepts connections (one thread each) until `stop` is set.
pub fn serve(listener: &TcpListener, stop: &AtomicBool, ws: &Arc<WsHandler>) {
    if listener.set_nonblocking(true).is_err() {
        return;
    }
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((s, _)) => {
                if s.set_nonblocking(false).is_err() {
                    continue;
                }
                let ws = Arc::clone(ws);
                std::thread::spawn(move || connection(s, &*ws));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One request on a fresh connection; the whole answer (the server closes after it).
    fn exchange(addr: SocketAddr, req: &str) -> String {
        let mut c = TcpStream::connect(addr).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        c.write_all(req.as_bytes()).unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        while let Ok(n) = c.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Runs a server on a free port, sends `req`, returns the answer.
    fn ask(req: &str) -> String {
        let listener = bind(0).unwrap();
        let addr = local_addr(&listener).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let st2 = Arc::clone(&stop);
        let ws: Arc<WsHandler> = Arc::new(|_, _| {});
        let h = std::thread::spawn(move || serve(&listener, &st2, &ws));
        // loopback connections made while other tests run are sometimes reset on Windows before
        // the server has seen them (the same exchange works on retry)
        let mut out = String::new();
        for _ in 0..10 {
            out = exchange(addr, req);
            if !out.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        stop.store(true, Ordering::Relaxed);
        h.join().unwrap();
        out
    }

    #[test]
    fn e13_03_index_ok() {
        let r = ask("GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 200"), "{r}");
        assert!(r.contains("<title>"));
        assert!(INDEX.contains("<title>"));
    }

    #[test]
    fn e13_03_health() {
        let r = ask("GET /health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 200"), "{r}");
        assert!(r.ends_with("ok"), "{r}");
        assert_eq!(route("GET", "/health?x=1").body, "ok");
    }

    #[test]
    fn e13_03_not_found() {
        let r = ask("GET /x HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 404"), "{r}");
        assert_eq!(route("POST", "/").status, 404);
        let r = ask("garbage\r\n\r\n");
        assert!(r.is_empty(), "{r}");
    }

    #[test]
    fn e13_03_localhost_only() {
        let l = bind(0).unwrap();
        let a = local_addr(&l).unwrap();
        assert!(a.ip().is_loopback());
        assert_eq!(a.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn e13_03_head_parse() {
        let h = parse_head(
            "GET /ws?a=1 HTTP/1.1\r\nHost: x\r\nUpgrade: WebSocket\r\nSec-WebSocket-Key: k",
        )
        .unwrap();
        assert!(h.is_ws_upgrade());
        assert_eq!(h.path(), "/ws");
        assert_eq!(h.header("sec-websocket-key"), Some("k"));
        assert!(parse_head("nope").is_none());
    }
}
