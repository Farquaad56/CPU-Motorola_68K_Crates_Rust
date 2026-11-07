//! `m68k-web`: local web front end server.
//!
//! Stops when `q` (or `quit`) is typed on standard input; a Ctrl-C ends the process and the
//! system closes the sockets (the crate forbids `unsafe`, which a signal handler would need).
#![forbid(unsafe_code)]

mod args;
mod http;
mod ws;

use std::io::BufRead;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Where the SingleStepTests files are read from.
const SST_DIR: &str = "tests/singlestep";

/// Line printed at start-up.
fn banner(addr: &str, roms: &Path) -> String {
    format!("m68k-web: http://{addr} (roms {})", roms.display())
}

/// Parses the arguments and binds the listening socket.
fn start(argv: &[String]) -> Result<(TcpListener, args::Args), String> {
    let a = args::parse_args(argv)?;
    let l =
        http::bind(a.port).map_err(|e| format!("cannot listen on 127.0.0.1:{}: {e}", a.port))?;
    Ok((l, a))
}

/// Serves until `stop` is set, then gives the connection threads time to close.
fn serve_until(listener: &TcpListener, roms: PathBuf, stop: &Arc<AtomicBool>) {
    let handler = ws::handler(roms, PathBuf::from(SST_DIR), Arc::clone(stop));
    http::serve(listener, stop, &handler);
    std::thread::sleep(Duration::from_millis(150));
}

fn real_main(argv: &[String]) -> Result<(), String> {
    let (listener, a) = start(argv)?;
    let addr = http::local_addr(&listener)
        .map_or_else(|| format!("127.0.0.1:{}", a.port), |s| s.to_string());
    println!("{}", banner(&addr, &a.roms_dir));
    println!("m68k-web: type q then Enter to stop");
    let stop = Arc::new(AtomicBool::new(false));
    let st = Arc::clone(&stop);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(l) if matches!(l.trim(), "q" | "quit") => {
                    st.store(true, Ordering::Relaxed);
                    return;
                }
                Ok(_) => {}
                Err(_) => return,
            }
        }
    });
    serve_until(&listener, a.roms_dir, &stop);
    println!("m68k-web: stopped");
    Ok(())
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = real_main(&argv) {
        eprintln!("m68k-web: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn e13_08_bind_error() {
        let taken = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = taken.local_addr().unwrap().port().to_string();
        let e = real_main(&v(&["--port", &port])).unwrap_err();
        assert!(e.contains("cannot listen on 127.0.0.1:"), "{e}");
        assert!(e.contains(&port), "{e}");
        // a bad argument is also reported, not a panic
        assert!(real_main(&v(&["--bogus"]))
            .unwrap_err()
            .contains("unknown argument"));
    }

    #[test]
    fn e13_08_shutdown_flag() {
        let (l, a) = start(&v(&["--port", "0"])).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let st = Arc::clone(&stop);
        let h = std::thread::spawn(move || serve_until(&l, a.roms_dir, &st));
        std::thread::sleep(Duration::from_millis(100));
        assert!(!h.is_finished());
        stop.store(true, Ordering::Relaxed);
        let t0 = std::time::Instant::now();
        h.join().unwrap();
        assert!(t0.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn e13_08_banner() {
        let b = banner("127.0.0.1:8080", Path::new("tests/roms"));
        assert!(b.contains("http://127.0.0.1:8080"), "{b}");
        assert!(b.contains("tests/roms"));
    }
}
