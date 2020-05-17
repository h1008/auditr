use std::path::Path;

use walkdir::WalkDir;

use anyhow::Result;

use crate::index::{Entry, HASH_INDEX_NAME, META_INDEX_NAME};

pub fn analyze_dir<T, R>(dir_name: &Path, compute_meta: bool, compute_hash: bool, mut update: T) -> Result<Vec<Entry>> where
    T: FnMut(u64) -> R {
    let mut entries = Vec::new();

    let hash_idx_path = dir_name.join(Path::new(HASH_INDEX_NAME));
    let meta_idx_path = dir_name.join(Path::new(META_INDEX_NAME));

    let x: [&Path; 2] = [hash_idx_path.as_path(), meta_idx_path.as_path()];
    let walk = WalkDir::new(dir_name).
        into_iter().
        filter_entry(|e| !x.contains(&e.path()));

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

pub fn total_file_size(dir_name: &Path) -> Result<u64> {
    let entries = analyze_dir(dir_name, true, false, |_| ())?;
    Ok(entries.iter().fold(0, |d, i| d + i.len))
}