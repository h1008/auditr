use std::{fmt, fs, io};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};
use sha2::digest::generic_array::functional::FunctionalSequence;

use anyhow::{anyhow, Result};

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
            hash: String::new(), // TODO: Optional?
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

    pub fn update_meta(&mut self, root: &Path) -> Result<()> {
        let path = root.join(&self.path);
        let meta = fs::metadata(path)?;
        let time = meta.modified()?.duration_since(UNIX_EPOCH)?;
        self.len = meta.len();
        self.modified = time.as_millis();
        Ok(())
    }

    pub fn update_hash(&mut self, root: &Path, force: bool) -> Result<()> {
        if force || self.hash.is_empty() {
            let path = root.join(&self.path);
            self.hash = Entry::hash_file(&path)?;
        }

        Ok(())
    }

    fn hash_file(file_name: &Path) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut file = File::open(file_name)?;
        let mut buf = [0; 4096];

        loop {
            let size = file.read(&mut buf)?;
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

    fn write_hash_line(&self, writer: &mut dyn Write) -> io::Result<()> {
        writeln!(writer, "{}  {}", self.hash, self)
    }

    fn write_meta_line(&self, writer: &mut dyn Write) -> io::Result<()> {
        writeln!(writer, "{}/{}  {}", self.modified, self.len, self)
    }
}

pub const HASH_INDEX_NAME: &str = ".checksums.sha256";
pub const META_INDEX_NAME: &str = ".checksums.meta";

pub fn index_exists(path: &Path) -> bool {
    let hash_index_file = path.join(HASH_INDEX_NAME);
    let meta_index_file = path.join(META_INDEX_NAME);
    hash_index_file.exists() || meta_index_file.exists()
}

pub fn load(path: &Path) -> Result<Vec<Entry>> {
    let hash_index = read_hash_index(&path.join(HASH_INDEX_NAME))?;
    let meta_index = read_meta_index(&path.join(META_INDEX_NAME))?;
    join_indices(hash_index, meta_index)
}

pub fn save(path: &Path, entries: &[Entry]) -> Result<()> {
    write_hash_index(&path.join(HASH_INDEX_NAME), &entries)?;
    write_meta_index(&path.join(META_INDEX_NAME), &entries)?;
    Ok(())
}

fn read_hash_index(file_name: &Path) -> Result<Vec<Entry>> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    let mut entries: Result<Vec<Entry>> = reader.lines().map(|line| {
        let line = line?;
        let line: Vec<&str> = line.split("  ").collect();
        Ok(Entry {
            path: PathBuf::from(line[1]),
            hash: String::from(line[0]),
            len: 0,
            modified: 0,
        })
    }).collect();

    if let Ok(ref mut e) = entries {
        e.sort_unstable()
    }

    entries
}

fn read_meta_index(file_name: &Path) -> Result<Vec<Entry>> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    let mut entries: Result<Vec<Entry>> = reader.lines().map(|line| {
        let line = line?;
        let line: Vec<&str> = line.split("  ").collect();
        let time_size: Vec<&str> = line[0].split('/').collect();

        Ok(Entry {
            path: PathBuf::from(line[1]),
            hash: String::new(),
            len: time_size[1].parse::<u64>()?,
            modified: time_size[0].parse::<u128>()?,
        })
    }).collect();

    if let Ok(ref mut e) = entries {
        e.sort_unstable()
    }

    entries
}

fn join_indices(hash_index: Vec<Entry>, meta_index: Vec<Entry>) -> Result<Vec<Entry>> {
    hash_index.iter().
        zip(meta_index.iter()).
        map(|(i1, i2)| {
            if i1.path != i2.path {
                return Err(anyhow!("Cannot join indices"));
            }
            Ok(Entry {
                path: i1.path.clone(),
                hash: i1.hash.clone(),
                len: i2.len,
                modified: i2.modified,
            })
        }).
        collect()
}

fn write_hash_index(file_name: &Path, entries: &[Entry]) -> io::Result<()> {
    let result = File::create(file_name)?;
    let mut writer = BufWriter::new(result);
    for t in entries {
        t.write_hash_line(&mut writer)?;
    }
    Ok(())
}

fn write_meta_index(file_name: &Path, entries: &[Entry]) -> io::Result<()> {
    let result = File::create(file_name)?;
    let mut writer = BufWriter::new(result);
    for t in entries {
        t.write_meta_line(&mut writer)?;
    }
    Ok(())
}