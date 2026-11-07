//! `m68k-web`: local web front end server (skeleton).
#![forbid(unsafe_code)]

mod args;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match args::parse_args(&argv) {
        Ok(a) => println!("m68k-web: port {}, roms {}", a.port, a.roms_dir.display()),
        Err(e) => {
            eprintln!("m68k-web: {e}");
            std::process::exit(1);
        }
    }
}
