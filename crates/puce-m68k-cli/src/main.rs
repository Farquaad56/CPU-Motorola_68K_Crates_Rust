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
        /// Only run files whose name contains this text.
        #[arg(long)]
        filter: Option<String>,
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
        Cmd::Run { rom } => commands::run(rom),
        Cmd::Ssc { dir, filter } => commands::ssc(dir, filter.as_deref()),
        Cmd::Disasm { file, org } => commands::disasm(file, org),
        Cmd::Fetch => commands::fetch(),
    };
    if let Err(e) = res {
        eprintln!("m68k: {e}");
        std::process::exit(1);
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
}
