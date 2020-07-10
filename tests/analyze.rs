extern crate auditr;

use std::path::{Path, PathBuf};

use anyhow::Result;
use mockall::*;
use mockall::predicate::always;
use tempfile::tempdir;

use auditr::analyze::{analyze_dir, total_file_size};
use auditr::filter::PathFilter;
pub use common::*;

mod common;

mock! {
    PathFilter {}
    trait PathFilter {
        fn matches(&self, e: &Path) -> bool;
    }
}

#[test]
fn test_analyze() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;
    given_file_with_random_contents(temp.path(), "a/f2.txt", 1024)?;
    given_file_with_random_contents(temp.path(), "b/f3.txt", 64)?;
    given_file_with_random_contents(temp.path(), "a4.txt", 16)?;
    given_file_with_random_contents(temp.path(), "c/f1.txt", 16)?;

    let mut filter = MockPathFilter::new();
    filter.expect_matches()
        .with(always())
        .returning(|e| !e.to_string_lossy().ends_with("/c"));

    // When
    let mut len = 0;
    let entries = analyze_dir(temp.path(), &filter, true, true, |l| len += l)?;

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
fn test_analyze_without_meta() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_random_contents(temp.path(), "a/f1.txt", 128)?;

    let filter = given_filter_accepting_all();

    // When
    let mut len = 0;
    let entries = analyze_dir(temp.path(), &filter, false, true, |l| len += l)?;

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

    let filter = given_filter_accepting_all();

    // When
    let mut called = 0;
    let entries = analyze_dir(temp.path(), &filter, true, false, |_| called += 1)?;

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
    given_file_with_random_contents(temp.path(), "c.txt", 128)?;

    let mut filter = MockPathFilter::new();
    filter.expect_matches()
        .with(always())
        .returning(|e| !e.to_string_lossy().ends_with("c.txt"));

    // When
    let size = total_file_size(temp.path(), &filter)?;

    // Then
    assert_eq!(size, 128 + 1024 + 64 + 16);

    Ok(())
}

fn given_filter_accepting_all() -> MockPathFilter {
    let mut filter = MockPathFilter::new();
    filter.expect_matches()
        .with(always())
        .returning(|_| true);
    filter
}