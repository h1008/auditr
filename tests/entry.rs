extern crate auditr;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use tempfile::tempdir;

use auditr::entry::Entry;
pub use common::*;

mod common;

#[test]
fn test_update_meta() -> Result<()> {
    // Given
    let temp = tempdir()?;
    let contents = "temp";

    fs::write(temp.path().join("test.txt"), contents)?;

    let mut e = Entry {
        path: PathBuf::from("test.txt"),
        hash: String::from(""),
        modified: 0,
        len: 0,
    };

    // When
    e.update_meta(temp.path())?;

    // Then
    assert_eq!(e.len, contents.len() as u64);
    assert!(e.modified > 0);

    Ok(())
}

#[test]
fn test_update_meta_non_existing_file() -> Result<()> {
    // Given
    let temp = tempdir()?;

    let mut e = Entry {
        path: PathBuf::from("does_not_exist.txt"),
        hash: String::from(""),
        modified: 0,
        len: 0,
    };

    // When
    let result = e.update_meta(temp.path());

    // Then
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_update_hash() -> Result<()> {
    // Given
    let temp = tempdir()?;

    fs::write(temp.path().join("test.txt"), "test")?;

    let mut e = Entry {
        path: PathBuf::from("test.txt"),
        hash: String::from(""),
        modified: 0,
        len: 0,
    };

    // When
    let mut len = 0u64;
    e.update_hash(temp.path(), false, |l| len += l)?;

    // Then
    assert_eq!(e.hash, "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    assert_eq!(len, 4);

    Ok(())
}

#[test]
fn test_update_hash_large_file() -> Result<()> {
    // Given
    let temp = tempdir()?;

    let file_size = 10 * 1024 * 1024;
    given_file_with_random_contents(temp.path(), "test.bin", file_size)?;

    let expected_hash = run_sha256sum(&temp.path().join("test.bin"))?;

    let mut e = Entry {
        path: PathBuf::from("test.bin"),
        hash: String::from(""),
        modified: 0,
        len: 0,
    };

    // When
    let mut len = 0u64;
    e.update_hash(temp.path(), false, |l| len += l)?;

    // Then
    assert_eq!(e.hash, expected_hash);
    assert_eq!(len, file_size as u64);

    Ok(())
}

#[test]
fn test_update_hash_no_update() -> Result<()> {
    // Given
    let temp = tempdir()?;

    let mut e = Entry {
        path: PathBuf::from("test.txt"),
        hash: String::from("existing_hash"),
        modified: 0,
        len: 0,
    };

    // When
    let mut len = 0u64;
    e.update_hash(temp.path(), false, |l| len += l)?;

    // Then
    assert_eq!(e.hash, "existing_hash");
    assert_eq!(len, 0);

    Ok(())
}

#[test]
fn test_update_hash_force() -> Result<()> {
    // Given
    let temp = tempdir()?;

    fs::write(temp.path().join("test.txt"), "test")?;

    let mut e = Entry {
        path: PathBuf::from("test.txt"),
        hash: String::from("existing_hash"),
        modified: 0,
        len: 0,
    };

    // When
    let mut len = 0u64;
    e.update_hash(temp.path(), true, |l| len += l)?;

    // Then
    assert_eq!(e.hash, "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    assert_eq!(len, 4);

    Ok(())
}

#[test]
fn test_update_hash_non_existing_file() -> Result<()> {
    // Given
    let temp = tempdir()?;

    let mut e = Entry {
        path: PathBuf::from("does_not_exist.txt"),
        hash: String::from(""),
        modified: 0,
        len: 0,
    };

    // When
    let result = e.update_hash(temp.path(), false, |_| ());

    // Then
    assert!(result.is_err());

    Ok(())
}
