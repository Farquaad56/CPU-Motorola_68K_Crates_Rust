//! `m68k-web`: local web front end server.
#![forbid(unsafe_code)]

mod args;
mod http;
mod ws;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = match args::parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("m68k-web: {e}");
            std::process::exit(1);
        }
    };
    let listener = match http::bind(a.port) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("m68k-web: {e}");
            std::process::exit(1);
        }
    };
    let addr = http::local_addr(&listener)
        .map_or_else(|| format!("127.0.0.1:{}", a.port), |s| s.to_string());
    println!("m68k-web: http://{addr} (roms {})", a.roms_dir.display());
    let stop = Arc::new(AtomicBool::new(false));
    let handler = ws::handler(
        a.roms_dir.clone(),
        PathBuf::from("tests/singlestep"),
        Arc::clone(&stop),
    );
    http::serve(&listener, &stop, &handler);
}
