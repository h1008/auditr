use std::io;
use std::io::{BufRead, stdout};
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use pbr::{ProgressBar, Units};

use crate::diff::diff_iter;
use crate::entry::Entry;
use crate::filter::DefaultPathFilter;
use crate::stats::Stats;

pub mod entry;
pub mod diff;
pub mod stats;
pub mod index;
pub mod analyze;
pub mod filter;

pub fn init(directory: &str) -> Result<i32> {
    let path = Path::new(directory);
    if index::index_exists(path) {
        bail!("An index already exists in this directory!");
    }

    let filter = filter::load_filter(path)?;
    let total = analyze::total_file_size(path, filter.as_ref())?;
    let pb_update = init_progress(total);

    let entries = analyze::analyze_dir(path, filter.as_ref(), true, true, pb_update)?;

    index::save(path, &entries)?;

    println!("{}", "Successfully initialized.".bold().green());

    Ok(0)
}

pub fn update(directory: &str) -> Result<i32> {
    let path = Path::new(directory);
    let entries = index::load(path, &DefaultPathFilter::new(path)).
        with_context(|| format!("No index found in directory '{}'", directory))?;

    let filter = filter::load_filter(path)?;
    let actual = analyze::analyze_dir(path, filter.as_ref(), true, false, |_| {})?;
    let it = diff_iter(entries.iter(), actual.iter(), Entry::compare_meta);

    let stats: Stats = it.collect();
    if !stats.modified() {
        println!("{}", "Nothing to update.".bold().green());
        return Ok(0);
    }

    show_stats(&stats);

    if !confirm("Continue? [N/y]")? {
        println!("{}", "Aborted.".bold().yellow());
        return Ok(0);
    }

    let total = stats.iter_new().
        filter(|e| e.hash.is_empty()).
        fold(0, |c, e| c + e.len);
    let mut pb_update = init_progress(total);

    let with_hash = |entry: &Entry| {
        let mut e = entry.clone();
        e.update_hash(path, false, &mut pb_update)?;
        Ok(e)
    };

    let mut updated_entries = stats.iter_new().
        map(with_hash).
        collect::<Result<Vec<Entry>>>()?;
    updated_entries.sort_unstable();

    index::save(path, &updated_entries)?;
    Ok(0)
}

pub fn audit(directory: &str, update: bool) -> Result<i32> {
    let path = Path::new(directory);
    let entries = index::load(path, &DefaultPathFilter::new(path))?;

    let filter = filter::load_filter(path)?;
    let total = analyze::total_file_size(path, filter.as_ref())?;
    let pb_update = init_progress(total);

    let actual = analyze::analyze_dir(path, filter.as_ref(), true, true, pb_update)?;

    let it = diff_iter(entries.iter(), actual.iter(), Entry::compare_hash_and_mtime);

    let stats: Stats = it.collect();

    show_stats(&stats);

    if !stats.updated_bitrot.is_empty() {
        println!("{}", "Audit failed - bitrot detected!".bold().red());

        if update {
            println!("Index was not updated.")
        }

        return Ok(3);
    }

    if stats.modified() {
        println!("{}", "Audit failed - difference detected!".bold().red());

        if update {
            index::save(path, &actual)?;
            println!("Index updated.");
        }

        return Ok(2);
    }

    println!("{}", "Audit successful.".bold().green());
    Ok(0)
}

fn confirm(msg: &str) -> Result<bool> {
    println!("{}", msg);

    let stdin = io::stdin();
    let mut str = String::new();
    stdin.lock().read_line(&mut str)?;

    Ok(str.eq_ignore_ascii_case("y\n"))
}

fn show_stats(stats: &Stats) {
    if stats.modified() {
        for s in stats.added.iter() {
            print_file("+", s);
        }
        for s in stats.updated.iter() {
            print_file("*", s);
        }
        for s in stats.updated_bitrot.iter() {
            print_file("!", s);
        }
        for s in stats.removed.iter() {
            print_file("-", s);
        }
        for (k, s) in stats.moved.iter() {
            let line = format!("[{}] {} (from {})", ">", s, k.to_string_lossy());
            println!("{}", line.yellow());
        }
    }

    println!();
    println!("{}", "====================================".dimmed());
    print_stat("New:", stats.added.len());
    print_stat("Updated:", stats.updated.len());
    print_stat("Updated (bitrot):", stats.updated_bitrot.len());
    print_stat("Removed:", stats.removed.len());
    print_stat("Moved:", stats.moved.len());
    print_stat("Unchanged:", stats.unchanged.len());
    print_stat("Total:", stats.total as usize);
    println!("{}", "====================================".dimmed());
    println!();
}

fn print_file(event: &str, entry: &Entry) {
    println!("{}", format!("[{}] {}", event, entry).yellow());
}

fn print_stat(name: &str, count: usize) {
    if count > 0 {
        println!("{:20}{:>16}", name.bold(), count);
    }
}

fn init_progress(total: u64) -> impl FnMut(u64) -> u64 {
    let is_a_tty = atty::is(atty::Stream::Stdout);

    let mut pb = ProgressBar::on(stdout(), total);
    pb.set_units(Units::Bytes);

    move |c| {
        if is_a_tty {
            pb.add(c)
        } else {
            0
        }
    }
}