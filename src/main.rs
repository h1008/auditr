use anyhow::Result;
use clap::Clap;

use auditr::*;

/// Auditr collects hashes and file system metadata of all files in a directory tree.
/// The collected data can be used at later point in time to detect changes (like files added, removed, or updated).
#[derive(Clap)]
#[clap(version = "0.1", author = "h1008")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Create the directory index
    #[clap(name = "init")]
    Init(Init),

    /// Update the directory index
    #[clap(name = "update")]
    Update(Update),

    /// Check for any changes compared to the directory index
    #[clap(name = "audit")]
    Audit(Audit),
}

/// Creates the directory index initially
#[derive(Clap)]
struct Init {
    directory: String
}

/// Updates the directory index
/// Show new, updated, deleted files according to metadata.
/// After confirmation, compute new index and commit
#[derive(Clap)]
struct Update {
    directory: String
}

/// Compares the directory's current state to the index and outputs the differences
#[derive(Clap)]
struct Audit {
    directory: String
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Init(u) => init(&u.directory),
        SubCommand::Update(u) => update(&u.directory),
        SubCommand::Audit(a) => audit(&a.directory)
    }
}