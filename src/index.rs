use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::io::BufRead;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::entry::Entry;
use crate::filter::PathFilter;

pub const HASH_INDEX_NAME: &str = ".checksums.sha256";
pub const META_INDEX_NAME: &str = ".checksums.meta";

pub fn index_exists(path: &Path) -> bool {
    let hash_index_file = path.join(HASH_INDEX_NAME);
    let meta_index_file = path.join(META_INDEX_NAME);
    hash_index_file.exists() || meta_index_file.exists()
}

pub fn load(path: &Path, filter: &dyn PathFilter) -> Result<Vec<Entry>> {
    let hash_index = read_hash_index(path, filter)?;
    let meta_index = read_meta_index(path, filter)?;
    join_indices(hash_index, meta_index)
}

pub fn save(path: &Path, entries: &[Entry]) -> Result<()> {
    write_hash_index(&path.join(HASH_INDEX_NAME), &entries)?;
    write_meta_index(&path.join(META_INDEX_NAME), &entries)?;
    Ok(())
}

fn read_hash_index(path: &Path, filter: &dyn PathFilter) -> Result<Vec<Entry>> {
    read_index(path, HASH_INDEX_NAME, filter, |line| {
        let line: Vec<&str> = line.splitn(2, "  ").collect();
        if line.len() != 2 {
            return Err(anyhow!("invalid hash index"));
        }

        Ok(Entry {
            path: PathBuf::from(line[1]),
            hash: String::from(line[0]),
            len: 0,
            modified: 0,
        })
    })
}

fn read_meta_index(path: &Path, filter: &dyn PathFilter) -> Result<Vec<Entry>> {
    read_index(path, META_INDEX_NAME, filter, |line| {
        let line: Vec<&str> = line.splitn(3, "  ").collect();
        if line.len() != 3 {
            bail!("meta index: invalid line format");
        }

        Ok(Entry {
            path: PathBuf::from(line[2]),
            hash: String::new(),
            len: line[1].parse::<u64>().
                map_err(|err| anyhow!("invalid meta format: invalid length: {}", err))?,
            modified: line[0].parse::<u128>().
                map_err(|err| anyhow!("invalid meta format: invalid modified timestamp: {}", err))?,
        })
    })
}

fn read_index<F>(path: &Path, file_name: &str, filter: &dyn PathFilter, mut f: F) -> Result<Vec<Entry>> where
    F: FnMut(String) -> Result<Entry> {
    let file = File::open(&path.join(file_name))?;
    let reader = BufReader::new(file);

    let mut entries: Result<Vec<Entry>> = reader.lines().
        map(|line| f(line?)).
        filter(|entry| {
            if let Ok(e) = entry {
                filter.matches(&path.join(e.path.as_path()))
            } else {
                true
            }
        }).collect();

    if let Ok(ref mut e) = entries {
        e.sort_unstable()
    }

    entries
}

fn join_indices(hash_index: Vec<Entry>, meta_index: Vec<Entry>) -> Result<Vec<Entry>> {
    if hash_index.len() != meta_index.len() {
        bail!("indices must have same number of entries");
    }

    hash_index.iter().
        zip(meta_index.iter()).
        map(|(i1, i2)| {
            if i1.path != i2.path {
                bail!("path of index entries do not match");
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
        writeln!(writer, "{}  {}", t.hash, t)?;
    }
    Ok(())
}

fn write_meta_index(file_name: &Path, entries: &[Entry]) -> io::Result<()> {
    let result = File::create(file_name)?;
    let mut writer = BufWriter::new(result);
    for t in entries {
        writeln!(writer, "{}  {}  {}", t.modified, t.len, t)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;
    use tempfile::tempdir;

    use crate::filter::DefaultPathFilter;

    use super::*;

    #[test]
    fn test_load() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_path = temp.path().join(HASH_INDEX_NAME);
        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/test_non_ascii_ß€%&².txt
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  test/with  spaces .txt
            ");
        fs::write(&hash_index_path, hash_index_contents)?;

        let meta_index_path = temp.path().join(META_INDEX_NAME);
        let meta_index_contents = indoc!("
            1578770227005  297742332  test/test_non_ascii_ß€%&².txt
            1225221568000  46738654  test/with  spaces .txt
            ");
        fs::write(&meta_index_path, meta_index_contents)?;

        // When
        let entries = load(temp.path(), &DefaultPathFilter::new(temp.path()))?;

        // Then
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].path.to_string_lossy(), "test/test_non_ascii_ß€%&².txt");
        assert_eq!(entries[0].hash, "9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85");
        assert_eq!(entries[0].len, 297742332);
        assert_eq!(entries[0].modified, 1578770227005);

        assert_eq!(entries[1].path.to_string_lossy(), "test/with  spaces .txt");
        assert_eq!(entries[1].hash, "048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544");
        assert_eq!(entries[1].len, 46738654);
        assert_eq!(entries[1].modified, 1225221568000);

        Ok(())
    }

    #[test]
    fn test_load_filter() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_path = temp.path().join(HASH_INDEX_NAME);
        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  .checksums.meta
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  .checksums.sha256
            0675e5e9efc82e1a795e61b616093adb13b6140b0f658d1f71ec8b9b733418fb  test/a.txt
            ");
        fs::write(&hash_index_path, hash_index_contents)?;

        let meta_index_path = temp.path().join(META_INDEX_NAME);
        let meta_index_contents = indoc!("
            1578770227005  297742332  .checksums.meta
            1225221568000  46738654  .checksums.sha256
            1771134938456  123492301  test/a.txt
            ");
        fs::write(&meta_index_path, meta_index_contents)?;

        // When
        let entries = load(temp.path(), &DefaultPathFilter::new(temp.path()))?;

        // Then
        assert_eq!(entries.len(), 1);

        assert_eq!(entries[0].path.to_string_lossy(), "test/a.txt");
        assert_eq!(entries[0].hash, "0675e5e9efc82e1a795e61b616093adb13b6140b0f658d1f71ec8b9b733418fb");
        assert_eq!(entries[0].len, 123492301);
        assert_eq!(entries[0].modified, 1771134938456);

        Ok(())
    }

    #[test]
    fn test_load_not_matching_files() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_path = temp.path().join(HASH_INDEX_NAME);
        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/a.txt
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  test/b.txt
            ");
        fs::write(&hash_index_path, hash_index_contents)?;

        let meta_index_path = temp.path().join(META_INDEX_NAME);
        let meta_index_contents = indoc!("
            1578770227005  297742332  test/a.txt
            1225221568000  46738654  test/c.txt
            ");
        fs::write(&meta_index_path, meta_index_contents)?;

        // When
        let result = load(temp.path(), &DefaultPathFilter::new(temp.path()));

        // Then
        assert_eq!(result.map_err(|e| e.to_string()), Err(String::from("path of index entries do not match")));

        Ok(())
    }

    #[test]
    fn test_load_different_entry_counts() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_path = temp.path().join(HASH_INDEX_NAME);
        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/a.txt
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  test/b.txt
            ");
        fs::write(&hash_index_path, hash_index_contents)?;

        let meta_index_path = temp.path().join(META_INDEX_NAME);
        let meta_index_contents = indoc!("
            1578770227005  297742332  test/a.txt
            ");
        fs::write(&meta_index_path, meta_index_contents)?;

        // When
        let result = load(temp.path(), &DefaultPathFilter::new(temp.path()));

        // Then
        assert_eq!(result.map_err(|e| e.to_string()), Err(String::from("indices must have same number of entries")));

        Ok(())
    }

    #[test]
    fn test_load_invalid_hash_index() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/a.txt
            INVALID
            ");
        fs::write(temp.path().join(HASH_INDEX_NAME), hash_index_contents)?;

        let meta_index_contents = indoc!("
            1578770227005  297742332  test/a.txt
            1225221568000  46738654  test/b.txt
            ");
        fs::write(temp.path().join(META_INDEX_NAME), meta_index_contents)?;

        // When
        let result = load(temp.path(), &DefaultPathFilter::new(temp.path()));

        // Then
        assert_eq!(result.map_err(|e| e.to_string()), Err(String::from("invalid hash index")));

        Ok(())
    }

    #[test]
    fn test_load_invalid_meta_index() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let hash_index_contents = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/a.txt
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  test/b.txt
            ");
        fs::write(temp.path().join(HASH_INDEX_NAME), hash_index_contents)?;

        let meta_index_contents = [
            indoc!("
                1578770227005  297742332  test/a.txt
                INVALID"),
            indoc!("1578770227005  test/a.txt"),
            indoc!("1578770227005  ABC297742332  test/a.txt"),
            indoc!("ABC1578770227005  297742332  test/a.txt"),
        ];

        for c in &meta_index_contents {
            fs::write(temp.path().join(META_INDEX_NAME), c)?;

            // When
            let result = load(temp.path(), &DefaultPathFilter::new(temp.path()));

            // Then
            assert!(result.is_err(), "expected error for index content: {:?}", c);
        }

        Ok(())
    }

    #[test]
    fn test_save() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let entries = [
            Entry {
                path: PathBuf::from("test/a.txt"),
                hash: String::from("9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85"),
                len: 297742332,
                modified: 1578770227005,
            },
            Entry {
                path: PathBuf::from("test/b.txt"),
                hash: String::from("048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544"),
                len: 46738654,
                modified: 1225221568000,
            }
        ];

        // When
        save(temp.path(), &entries)?;

        // Then
        let expected_hash_index_content = indoc!("
            9489d28fbd325690224dd76c0d7ae403177e15a0d63758cc0171327b5ba2aa85  test/a.txt
            048287162a3a9e8976f0aec50af82965c7c622d479bcf15f4db2d67358bd0544  test/b.txt
            ");
        let result = fs::read_to_string(temp.path().join(HASH_INDEX_NAME))?;
        assert_eq!(result, expected_hash_index_content);

        let expected_meta_index_content = indoc!("
            1578770227005  297742332  test/a.txt
            1225221568000  46738654  test/b.txt
            ");
        let result = fs::read_to_string(temp.path().join(META_INDEX_NAME))?;
        assert_eq!(result, expected_meta_index_content);

        Ok(())
    }

    #[test]
    fn test_index_exists_no_index() -> Result<()> {
        // Given
        let temp = tempdir()?;

        // When
        let exists = index_exists(temp.path());

        // Then
        assert!(!exists);

        Ok(())
    }

    #[test]
    fn test_index_exists_only_hash_index() -> Result<()> {
        // Given
        let temp = tempdir()?;
        fs::write(temp.path().join(HASH_INDEX_NAME), "")?;

        // When
        let exists = index_exists(temp.path());

        // Then
        assert!(exists);

        Ok(())
    }

    #[test]
    fn test_index_exists_only_meta_index() -> Result<()> {
        // Given
        let temp = tempdir()?;
        fs::write(temp.path().join(META_INDEX_NAME), "")?;

        // When
        let exists = index_exists(temp.path());

        // Then
        assert!(exists);

        Ok(())
    }

    #[test]
    fn test_index_exists_both_index_files() -> Result<()> {
        // Given
        let temp = tempdir()?;
        fs::write(temp.path().join(HASH_INDEX_NAME), "")?;
        fs::write(temp.path().join(META_INDEX_NAME), "")?;

        // When
        let exists = index_exists(temp.path());

        // Then
        assert!(exists);

        Ok(())
    }
}
