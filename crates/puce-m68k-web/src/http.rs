//! Minimal HTTP server: the embedded page, `/health`, 404 for the rest. 127.0.0.1 only.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server};

/// The page served on `GET /`.
pub const INDEX: &str = include_str!("../../../docs/frontend/index.html");

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

/// Binds 127.0.0.1:`port` (0 = any free port).
pub fn bind(port: u16) -> Result<Server, String> {
    Server::http(("127.0.0.1", port)).map_err(|e| e.to_string())
}

/// Address the server listens on.
pub fn local_addr(server: &Server) -> Option<SocketAddr> {
    server.server_addr().to_ip()
}

/// Answers requests until `stop` is set.
pub fn serve(server: &Server, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        let Ok(Some(rq)) = server.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };
        let method = match rq.method() {
            Method::Get => "GET",
            Method::Head => "HEAD",
            _ => "OTHER",
        };
        let r = route(method, rq.url());
        let mut resp = Response::from_string(if method == "HEAD" { "" } else { r.body })
            .with_status_code(r.status);
        if let Ok(h) = Header::from_bytes("Content-Type", r.ctype) {
            resp.add_header(h);
        }
        let _ = rq.respond(resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::Arc;

    /// One request on a fresh connection; empty when the connection was reset.
    fn exchange(addr: SocketAddr, req: &str) -> String {
        let Ok(mut c) = TcpStream::connect(addr) else {
            return String::new();
        };
        if c.write_all(req.as_bytes()).is_err() {
            return String::new();
        }
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        // stop as soon as the announced body is complete: Windows may reset the connection
        // after the answer
        while let Ok(n) = c.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            let s = String::from_utf8_lossy(&buf).into_owned();
            if let Some((head, body)) = s.split_once("\r\n\r\n") {
                let len = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if body.len() >= len {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Runs a server on a free port, sends `req` (retried: a connection made right after the
    /// bind can be reset on Windows), returns the whole answer.
    fn ask(req: &str) -> (SocketAddr, String) {
        let server = Arc::new(bind(0).unwrap());
        let addr = local_addr(&server).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (s2, st2) = (Arc::clone(&server), Arc::clone(&stop));
        let h = std::thread::spawn(move || serve(&s2, &st2));
        let mut out = String::new();
        for _ in 0..10 {
            out = exchange(addr, req);
            if !out.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        stop.store(true, Ordering::Relaxed);
        h.join().unwrap();
        (addr, out)
    }
    #[test]
    fn e13_03_index_ok() {
        let (_, r) = ask("GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 200"), "{r}");
        assert!(r.contains("<title>"));
        assert!(INDEX.contains("<title>"));
    }

    #[test]
    fn e13_03_health() {
        let (_, r) = ask("GET /health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 200"), "{r}");
        assert!(r.ends_with("ok"), "{r}");
        assert_eq!(route("GET", "/health?x=1").body, "ok");
    }

    #[test]
    fn e13_03_not_found() {
        let (_, r) = ask("GET /x HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
        assert!(r.starts_with("HTTP/1.1 404"), "{r}");
        assert_eq!(route("POST", "/").status, 404);
    }

    #[test]
    fn e13_03_localhost_only() {
        let server = bind(0).unwrap();
        let a = local_addr(&server).unwrap();
        assert!(a.ip().is_loopback());
        assert_eq!(a.ip().to_string(), "127.0.0.1");
    }
}
