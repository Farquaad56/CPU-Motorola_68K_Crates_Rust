//! `m68k`: command-line tool (test suites, disassembler).
#![forbid(unsafe_code)]

mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Command line of the `m68k` tool.
#[derive(Parser, Debug)]
#[command(name = "m68k", version, about = "puce-m68k command-line tool")]
pub struct Cli {
    /// Sub-command to run.
    #[command(subcommand)]
    pub command: Cmd,
}

/// Available sub-commands.
#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Run a program on the flat RAM machine.
    Run {
        /// Program file.
        rom: PathBuf,
    },
    /// Run the SingleStepTests/m68000 files of a directory.
    Ssc {
        /// Directory containing `*.json.bin` files.
        dir: PathBuf,
        /// Only run this file (`ADD.b`) or family (`ADD` = `ADD.*`); repeatable.
        #[arg(long)]
        filter: Vec<String>,
        /// Only run the instructions of this epic (reads docs/specs/data/instructions.tsv).
        #[arg(long)]
        epic: Option<String>,
        /// Exit with 1 if a file that is not a known exception has failures.
        #[arg(long)]
        strict: bool,
        /// Stop after this many failures in total.
        #[arg(long)]
        max_fail: Option<u32>,
        /// Known exceptions file (default docs/specs/data/sst_known_exceptions.txt).
        #[arg(long)]
        known: Option<PathBuf>,
    },
    /// Disassemble a binary file.
    Disasm {
        /// Binary file.
        file: PathBuf,
        /// Origin address (hex).
        #[arg(long, default_value = "0")]
        org: String,
    },
    /// Download the reference documents and test suites.
    Fetch,
}

fn main() {
    let cli = Cli::parse();
    let res = match &cli.command {
        Cmd::Run { rom } => commands::run(rom).map(|()| 0),
        Cmd::Ssc {
            dir,
            filter,
            epic,
            strict,
            max_fail,
            known,
        } => commands::ssc(&commands::SscArgs {
            dir,
            filters: filter,
            epic: epic.as_deref(),
            strict: *strict,
            max_fail: *max_fail,
            known: known.as_deref(),
        }),
        Cmd::Disasm { file, org } => commands::disasm(file, org).map(|()| 0),
        Cmd::Fetch => commands::fetch().map(|()| 0),
    };
    match res {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("m68k: {e}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e01_03_parse_run() {
        let c = Cli::try_parse_from(["m68k", "run", "x.bin"]).unwrap();
        assert!(matches!(c.command, Cmd::Run { .. }));
    }

    #[test]
    fn e01_03_unknown_cmd() {
        assert!(Cli::try_parse_from(["m68k", "bogus"]).is_err());
    }

    #[test]
    fn e03_07_parse_ssc() {
        let c = Cli::try_parse_from([
            "m68k",
            "ssc",
            "tests/singlestep",
            "--filter",
            "NOP",
            "--strict",
        ])
        .unwrap();
        match c.command {
            Cmd::Ssc {
                dir,
                filter,
                strict,
                epic,
                max_fail,
                known,
            } => {
                assert_eq!(dir, PathBuf::from("tests/singlestep"));
                assert_eq!(filter, ["NOP"]);
                assert!(strict && epic.is_none() && max_fail.is_none() && known.is_none());
            }
            c => panic!("{c:?}"),
        }
    }

    #[test]
    fn e03_07_parse_epic() {
        let c = Cli::try_parse_from([
            "m68k",
            "ssc",
            "d",
            "--epic",
            "E06",
            "--max-fail",
            "10",
            "--known",
            "k.txt",
        ])
        .unwrap();
        match c.command {
            Cmd::Ssc {
                epic,
                max_fail,
                known,
                ..
            } => {
                assert_eq!(epic.as_deref(), Some("E06"));
                assert_eq!(max_fail, Some(10));
                assert_eq!(known, Some(PathBuf::from("k.txt")));
            }
            c => panic!("{c:?}"),
        }
    }

    #[test]
    fn e03_07_parse_filters() {
        let c = Cli::try_parse_from(["m68k", "ssc", "d", "--filter", "ADD", "--filter", "MOVE.w"])
            .unwrap();
        match c.command {
            Cmd::Ssc { filter, .. } => assert_eq!(filter, ["ADD", "MOVE.w"]),
            c => panic!("{c:?}"),
        }
    }

    #[test]
    fn e03_07_bad_flag() {
        assert!(Cli::try_parse_from(["m68k", "ssc", "d", "--bogus"]).is_err());
    }
}
