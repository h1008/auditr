use std::path::Path;

use walkdir::WalkDir;

use anyhow::Result;

use crate::index::{Entry, HASH_INDEX_NAME, META_INDEX_NAME};

pub fn analyze_dir(dir_name: &Path, compute_hash: bool) -> Result<Vec<Entry>> {
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
        e.update_meta(dir_name)?;

        if compute_hash {
            e.update_hash(dir_name, true)?;
        }

        entries.push(e)
    }

    entries.sort_unstable();

    Ok(entries)
}
