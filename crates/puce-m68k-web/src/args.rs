//! Command-line arguments of `m68k-web` (hand-written parser, no clap).

use std::path::PathBuf;

/// Parsed arguments.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// TCP port on 127.0.0.1.
    pub port: u16,
    /// Directory with test programs.
    pub roms_dir: PathBuf,
}

/// Parse `--port N`, `--roms DIR`.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args {
        port: 8080,
        roms_dir: PathBuf::from("tests/roms"),
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                let v = args.get(i).ok_or("--port needs a value")?;
                out.port = v.parse::<u16>().map_err(|_| format!("bad port: {v}"))?;
            }
            "--roms" => {
                i += 1;
                out.roms_dir = PathBuf::from(args.get(i).ok_or("--roms needs a value")?);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn e01_04_defaults() {
        let a = parse_args(&[]).unwrap();
        assert_eq!(a.port, 8080);
        assert_eq!(a.roms_dir, PathBuf::from("tests/roms"));
    }

    #[test]
    fn e01_04_port_ok() {
        assert_eq!(parse_args(&v(&["--port", "9000"])).unwrap().port, 9000);
    }

    #[test]
    fn e01_04_port_bad() {
        assert!(parse_args(&v(&["--port", "abc"])).is_err());
    }

    #[test]
    fn e01_04_unknown_flag() {
        assert!(parse_args(&v(&["--foo"])).is_err());
    }
}
