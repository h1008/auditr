use std::process;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use auditr::*;

/// Auditr collects hashes and file system metadata of all files in a directory tree.
/// The collected data can be used at later point in time to detect changes (like files added, removed, or updated).
#[derive(Parser)]
#[clap(version = "0.2.0", author = "h1008")]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Creates the directory index initially
    #[command(name = "init")]
    Init {
        directory: String
    },

    /// Updates the directory index
    /// Show new, updated, deleted files according to metadata.
    /// After confirmation, compute new index and commit
    #[command(name = "update")]
    Update {
        directory: String,
    },

    /// Compares the directory's current state to the index and outputs the differences
    #[command(name = "audit")]
    Audit {
        directory: String,

        /// Update the index after audit unless bitrot was detected.
        #[arg(short, long)]
        update: bool,
    },
}

fn run() -> Result<i32> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Init {directory} => init(&directory),
        SubCommand::Update {directory} => update(&directory),
        SubCommand::Audit {directory, update} => audit(&directory, update)
    }
}

fn main() {
    process::exit(match run() {
        Ok(ret) => ret,
        Err(err) => {
            eprintln!("{}", err.to_string().bold().red());
            1
        }
    });
}