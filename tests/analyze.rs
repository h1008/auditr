extern crate auditr;

use std::path::PathBuf;

use anyhow::Result;
use tempfile::tempdir;

use auditr::analyze::{analyze_dir, total_file_size};
use auditr::index::{HASH_INDEX_NAME, META_INDEX_NAME};
pub use common::*;

mod common;

#[test]
fn test_analyze() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;
    given_file_with_random_contents(temp.path(), "a/f2.txt", 1024)?;
    given_file_with_random_contents(temp.path(), "b/f3.txt", 64)?;
    given_file_with_random_contents(temp.path(), "a4.txt", 16)?;

    // When
    let mut len = 0;
    let entries = analyze_dir(temp.path(), true, true, |l| len += l)?;

    // Then
    assert_eq!(entries.len(), 4);

    assert_eq!(entries[0].path, PathBuf::from("a/f1.txt"));
    assert_eq!(entries[0].hash.is_empty(), false);
    assert_eq!(entries[0].len, 128);
    assert_ne!(entries[0].modified, 0);

    assert_eq!(entries[1].path, PathBuf::from("a/f2.txt"));
    assert_eq!(entries[1].hash.is_empty(), false);
    assert_eq!(entries[1].len, 1024);
    assert_ne!(entries[1].modified, 0);

    assert_eq!(entries[2].path, PathBuf::from("a4.txt"));
    assert_eq!(entries[2].hash.is_empty(), false);
    assert_eq!(entries[2].len, 16);
    assert_ne!(entries[2].modified, 0);

    assert_eq!(entries[3].path, PathBuf::from("b/f3.txt"));
    assert_eq!(entries[3].hash.is_empty(), false);
    assert_eq!(entries[3].len, 64);
    assert_ne!(entries[3].modified, 0);

    assert_eq!(len, 128 + 1024 + 64 + 16);

    Ok(())
}

#[test]
fn test_analyze_exclude_index_files() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), HASH_INDEX_NAME, 128)?;
    given_file_with_random_contents(temp.path(), META_INDEX_NAME, 128)?;

    // When
    let mut called = 0;
    let entries = analyze_dir(temp.path(), true, true, |_| called += 1)?;

    // Then
    assert_eq!(entries.len(), 0);
    assert_eq!(called, 0);

    Ok(())
}

#[test]
fn test_analyze_without_meta() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;

    // When
    let mut len = 0;
    let entries = analyze_dir(temp.path(), false, true, |l| len += l)?;

    // Then
    assert_eq!(entries.len(), 1);

    assert_eq!(entries[0].len, 0);
    assert_eq!(entries[0].modified, 0);

    assert_eq!(len, 128);

    Ok(())
}

#[test]
fn test_analyze_without_hash() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;

    // When
    let mut called = 0;
    let entries = analyze_dir(temp.path(), true, false, |_| called += 1)?;

    // Then
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].hash.is_empty(), true);
    assert_eq!(called, 0);

    Ok(())
}

#[test]
fn test_total_file_size() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;
    given_file_with_random_contents(temp.path(), "a/f2.txt", 1024)?;
    given_file_with_random_contents(temp.path(), "b/f3.txt", 64)?;
    given_file_with_random_contents(temp.path(), "a4.txt", 16)?;
    given_file_with_random_contents(temp.path(), HASH_INDEX_NAME, 128)?;
    given_file_with_random_contents(temp.path(), META_INDEX_NAME, 128)?;

    // When
    let size = total_file_size(temp.path())?;

    // Then
    assert_eq!(size, 128 + 1024 + 64 + 16);

    Ok(())
}