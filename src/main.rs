use std::io;
use std::io::{BufRead, stderr};
use std::path::Path;

use clap::Clap;

use anyhow::{bail, Result, Context};

use crate::diff::diff_iter;
use crate::index::Entry;
use crate::stats::Stats;
use pbr::{ProgressBar, Units};

mod index;
mod analyze;
mod diff;
mod stats;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = "0.1", author = "h1008")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,

    // TODO: --progress
    // TODO: --exclude
    // TODO: --yes
    // TODO: --verbose
}

#[derive(Clap)]
enum SubCommand {
    /// Create the directory index
    #[clap(name = "init")]
    Init(Init),

    /// Update the directory index
    #[clap(name = "update")]
    Update(Update),

    /// A help message for the Audit subcommand
    #[clap(name = "audit")]
    Audit(Audit),
}

/// A subcommand for controlling testing
#[derive(Clap)]
struct Init {
    directory: String
}

/// Create or update the directory index
/// Show new, updated, deleted files according to metadata (if verbose) or stats (else)
/// After confirmation, compute new index and commit
#[derive(Clap)]
struct Update {
    directory: String
}

/// A subcommand for controlling testing
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

    // TODO: colored output
    // TODO: run https://github.com/rust-lang/rust-clippy
    // TODO: https://github.com/ssokolow/rust-cli-boilerplate
    // TODO: Tests, Integration
    // TODO: error handling (with_context)
    // TODO: optimize speed
    // TODO: make (?) file (build, build/release, lint, run, test, ..)
    // https://github.com/rust-unofficial/awesome-rust
}

fn init(directory: &str) -> Result<()> {
    eprintln!("Initializing indices in '{}'...", directory);

    let path = Path::new(directory);
    if index::index_exists(path) {
        bail!("An index already exists in this directory!");
    }

    let total = analyze::total_file_size(path)?;
    let mut pb = ProgressBar::on(stderr(), total as u64);
    pb.set_units(Units::Bytes);

    let entries = analyze::analyze_dir(path, true, true, |c| pb.add(c))?;
    pb.finish_print("Done.");

    index::save(path, &entries)
}

fn update(directory: &str) -> Result<()> {
    eprintln!("Updating indices in directory '{}'...", directory);

    let path = Path::new(directory);
    let entries = index::load(path).
        with_context(|| format!("No index found in directory '{}'", directory))?;

    let actual = analyze::analyze_dir(path, true, false, |_| {})?;
    let it = diff_iter(entries.iter(), actual.iter(), Entry::compare_meta);

    let stats: Stats = it.collect();
    if !stats.modified() {
        println!("Nothing to update.");
        return Ok(());
    }

    show_stats(&stats);

    eprintln!("Continue? [N/y]");

    let stdin = io::stdin();
    let mut str = String::new();
    stdin.lock().read_line(&mut str)?;

    // TODO: check yes

    let total = stats.iter_new().fold(0, |c, e| c + e.len);
    let mut pb = ProgressBar::on(stderr(), total as u64);
    pb.set_units(Units::Bytes);

    let with_hash = |entry: &Entry| {
        let mut e = entry.clone();
        e.update_hash(path, false, |c| pb.add(c))?;
        Ok(e)
    };

    let mut updated_entries = stats.iter_new().
        map(with_hash).
        collect::<Result<Vec<Entry>>>()?;
    updated_entries.sort_unstable();
    pb.finish_print("Done.");

    index::save(path, &updated_entries)
}

fn audit(directory: &str) -> Result<()> {
    eprintln!("Running audit in directory '{}'...", directory);

    let path = Path::new(directory);
    let entries = index::load(path)?;

    let total = analyze::total_file_size(path)?;
    let mut pb = ProgressBar::on(stderr(), total as u64);
    pb.set_units(Units::Bytes);

    let actual = analyze::analyze_dir(path, true, true, |c| pb.add(c))?;
    pb.finish_print("Done.");

    let it = diff_iter(entries.iter(), actual.iter(), Entry::compare_hash);

    let stats: Stats = it.collect();

    show_stats(&stats);

    if stats.modified() {
        bail!("Audit failed - difference detected!");
    }

    println!("Audit successful");

    Ok(())
}

fn show_stats(stats: &Stats) {
    if stats.modified() {
        println!("Files");
        println!("New (+), deleted (-), moved (>), updated (*), updated but with same modified timestamp (!)");
        println!();
        for s in stats.added.iter() {
            println!("[+] {}", s);
        }
        for s in stats.updated.iter() {
            println!("[*] {}", s);
        }
        for s in stats.updated_bitrot.iter() {
            println!("[!] {}", s);
        }
        for s in stats.removed.iter() {
            println!("[-] {}", s);
        }
        for (k, s) in stats.moved.iter() {
            println!("[>] {} (from {})", s, k.to_string_lossy());
        }
    }

    println!();
    println!("====================================");
    println!("Stats");
    println!("------------------------------------");
    println!("New:                {:>16}", stats.added.len());
    println!("Updated:            {:>16}", stats.updated.len());
    println!("Updated (bitrot):   {:>16}", stats.updated_bitrot.len());
    println!("Removed:            {:>16}", stats.removed.len());
    println!("Moved:              {:>16}", stats.moved.len());
    println!("Unchanged:          {:>16}", stats.unchanged.len());
    println!("Total:              {:>16}", stats.total);
    println!("====================================");
    println!();
}
