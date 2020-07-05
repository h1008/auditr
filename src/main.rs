use std::io;
use std::io::{BufRead, stderr};
use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Clap;
use pbr::{ProgressBar, Units};

use crate::diff::diff_iter;
use crate::index::Entry;
use crate::stats::Stats;

mod index;
mod analyze;
mod diff;
mod stats;

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

fn confirm(msg: &str) -> Result<bool> {
    eprintln!("{}", msg);

    let stdin = io::stdin();
    let mut str = String::new();
    stdin.lock().read_line(&mut str)?;

    Ok(str.eq_ignore_ascii_case("y\n"))
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

    show_stats(&stats, false);

    if !confirm("Continue? [N/y]")? {
        eprintln!("Aborted.");
        return Ok(());
    }

    let total = stats.iter_new().
        filter(|e| e.hash.is_empty()).
        fold(0, |c, e| c + e.len);
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

    show_stats(&stats, true);

    if stats.modified() {
        bail!("Audit failed - difference detected!");
    }

    println!("Audit successful");

    Ok(())
}

fn show_stats(stats: &Stats, audit: bool) {
    if stats.modified() {
        println!("Files");
        if audit {
            println!("New (+), deleted (-), moved (>), updated (*), updated but with same modified timestamp (!)");
        } else {
            println!("New (+), deleted (-), updated (*)");
        }
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
    print_stat("New:", stats.added.len());
    print_stat("Updated:", stats.updated.len());
    print_stat("Updated (bitrot):", stats.updated_bitrot.len());
    print_stat("Removed:", stats.removed.len());
    print_stat("Moved:", stats.moved.len());
    print_stat("Unchanged:", stats.unchanged.len());
    print_stat("Total:", stats.total as usize);
    println!("====================================");
    println!();
}

fn print_stat(name: &str, count: usize) {
    if count > 0 {
        println!("{:20}{:>16}", name, count);
    }
}