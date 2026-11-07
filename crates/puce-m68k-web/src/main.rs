//! `m68k-web`: local web front end server.
#![forbid(unsafe_code)]

mod args;
mod http;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = match args::parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("m68k-web: {e}");
            std::process::exit(1);
        }
    };
    let server = match http::bind(a.port) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("m68k-web: {e}");
            std::process::exit(1);
        }
    };
    let addr = http::local_addr(&server)
        .map_or_else(|| format!("127.0.0.1:{}", a.port), |s| s.to_string());
    println!("m68k-web: http://{addr} (roms {})", a.roms_dir.display());
    http::serve(&server, &std::sync::atomic::AtomicBool::new(false));
}
