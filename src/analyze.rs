use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use crate::entry::Entry;
use crate::filter::PathFilter;

pub fn analyze_dir<T, R>(dir_name: &Path, filter: &dyn PathFilter, compute_meta: bool, compute_hash: bool, mut update: T) -> Result<Vec<Entry>> where
    T: FnMut(u64) -> R {
    let mut entries = Vec::new();

    let walk = WalkDir::new(dir_name).
        into_iter().
        filter_entry(|e| filter.matches(e.path()));

    for entry in walk {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().strip_prefix(dir_name)?;
        let mut e = Entry::from_path(path);

        if compute_meta {
            e.update_meta(dir_name)?;
        }

        if compute_hash {
            e.update_hash(dir_name, true, &mut update)?;
        }

        entries.push(e)
    }

    entries.sort_unstable();

    Ok(entries)
}

pub fn total_file_size(dir_name: &Path, filter: &dyn PathFilter) -> Result<u64> {
    let entries = analyze_dir(dir_name, filter, true, false, |_| ())?;
    Ok(entries.iter().fold(0, |d, i| d + i.len))
}