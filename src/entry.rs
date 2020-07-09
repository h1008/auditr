use std::{fmt, fs};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::Result;
use sha2::{Digest, Sha256};
use sha2::digest::generic_array::functional::FunctionalSequence;

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub hash: String,
    pub len: u64,
    pub modified: u128,
}

impl Display for Entry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let path = self.path.to_str().unwrap_or("-");
        write!(f, "{}", path)
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Entry {}

impl Entry {
    pub fn from_path(path: &Path) -> Entry {
        Entry {
            path: path.to_path_buf(),
            hash: String::new(),
            len: 0,
            modified: 0,
        }
    }

    pub fn compare_meta(e1: &Entry, e2: &Entry) -> bool {
        e1.len == e2.len && e1.modified == e2.modified
    }

    pub fn compare_hash(e1: &Entry, e2: &Entry) -> bool {
        e1.hash == e2.hash
    }

    pub fn compare_hash_and_mtime(e1: &Entry, e2: &Entry) -> bool {
        e1.modified == e2.modified && e1.hash == e2.hash
    }

    pub fn update_meta(&mut self, root: &Path) -> Result<()> {
        let path = root.join(&self.path);
        let meta = fs::metadata(path)?;
        let time = meta.modified()?.duration_since(UNIX_EPOCH)?;
        self.len = meta.len();
        self.modified = time.as_millis();
        Ok(())
    }

    pub fn update_hash<T, R>(&mut self, root: &Path, force: bool, update: &mut T) -> Result<()> where
        T: FnMut(u64) -> R {
        if force || self.hash.is_empty() {
            let path = root.join(&self.path);
            self.hash = Entry::hash_file(&path, update)?;
        }

        Ok(())
    }

    fn hash_file<T, R>(file_name: &Path, update: &mut T) -> Result<String> where
        T: FnMut(u64) -> R {
        let mut hasher = Sha256::new();
        let mut file = File::open(file_name)?;
        let mut buf = [0; 1024 * 1024];

        loop {
            let size = file.read(&mut buf)?;
            update(size as u64);
            if size != buf.len() {
                hasher.input(&buf[0..size]);
                break;
            }

            hasher.input(&buf[..]);
        }

        Ok(hasher.result().
            map(|b| format!("{:02x}", b)).
            to_vec().join(""))
    }
}