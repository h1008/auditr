use std::io;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{bail, Result};
use indoc::indoc;
use tempfile::tempdir;

use auditr::filter::globfilter::GLOB_FILTER_FILENAME;
pub use common::*;

mod common;

const BINARY_PATH: &str = "target/debug/auditr";

#[test]
fn test_init() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_without_index(temp.path())?;

    // When
    let result = run_init(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 0);
    assert!(temp.path().join(".checksums.sha256").exists());
    assert!(temp.path().join(".checksums.meta").exists());

    Ok(())
}

#[test]
fn test_init_twice() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_index(temp.path())?;

    // When
    let result = run_init(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 1);

    Ok(())
}

#[test]
fn test_audit_no_changes() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_index(temp.path())?;

    // When
    let result = run_audit(temp.path())?;

    // Then
    let out = stdout(&result);
    assert_eq!(status_code(&result), 0);
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+6$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+6$"));
    assert!(out.contains("Audit successful"));

    Ok(())
}

#[test]
fn test_audit_modified() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_modified_index(temp.path(), false)?;

    // When
    let result = run_audit(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 2);

    let out = stdout(&result);
    assert!(match_regex(&out, r"(?m)^New:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Updated:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Removed:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Moved:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+6$"));

    assert!(out.contains("[+] a/new.txt"));
    assert!(out.contains("[*] f1.txt"));
    assert!(out.contains("[*] a/f2b.txt"));
    assert!(out.contains("[-] a/b/f3.txt"));
    assert!(out.contains("[>] a/large_new.txt (from c/large.txt)"));

    Ok(())
}

#[test]
fn test_audit_modified_bitrot() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_modified_index(temp.path(), true)?;

    // When
    let result = run_audit(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 3);

    let out = stdout(&result);
    assert!(match_regex(&out, r"(?m)^Updated \(bitrot\):\s+1$"));

    assert!(out.contains("[!] a/f2a.txt"));

    Ok(())
}

#[test]
fn test_audit_without_index() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_without_index(temp.path())?;

    // When
    let result = run_audit(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 1);

    Ok(())
}

#[test]
fn test_update() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_modified_index(temp.path(), true)?;

    // When
    let result = run_update(temp.path(), true)?;

    // Then
    assert_eq!(status_code(&result), 0);

    let out = stdout(&result);
    assert!(match_regex(&out, r"(?m)^New:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Updated:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Updated \(bitrot\):\s+1$"));
    assert!(match_regex(&out, r"(?m)^Removed:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+6$"));

    assert!(out.contains("[+] a/new.txt"));
    assert!(out.contains("[+] a/large_new.txt"));
    assert!(out.contains("[*] f1.txt"));
    assert!(out.contains("[!] a/f2a.txt"));
    assert!(out.contains("[-] a/b/f3.txt"));
    assert!(out.contains("[-] c/large.txt"));

    let result = run_audit(temp.path())?;
    assert_eq!(status_code(&result), 0);

    Ok(())
}

#[test]
fn test_update_without_changes() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_index(temp.path())?;

    // When
    let result = run_update(temp.path(), true)?;

    // Then
    assert_eq!(status_code(&result), 0);
    assert!(stdout(&result).contains("Nothing to update."));

    Ok(())
}

#[test]
fn test_update_abort() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_with_modified_index(temp.path(), false)?;

    // When
    let result = run_update(temp.path(), false)?;

    // Then
    assert_eq!(status_code(&result), 0);

    let result = run_audit(temp.path())?;
    assert_eq!(status_code(&result), 2);

    Ok(())
}

#[test]
fn test_update_without_index() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_dir_without_index(temp.path())?;

    // When
    let result = run_update(temp.path(), true)?;

    // Then
    assert_eq!(status_code(&result), 1);

    Ok(())
}

#[test]
fn test_filter_init_audit() -> Result<()> {
    // Given
    let temp = tempdir()?;

    given_file_with_contents(temp.path(), GLOB_FILTER_FILENAME, indoc!("
        + a/b*
        - a/**
    "))?;
    given_dir_with_index(temp.path())?;

    // When
    let result = run_audit(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 0);

    let out = stdout(&result);
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+4$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+4$"));

    Ok(())
}

#[test]
fn test_filter_update() -> Result<()> {
    // Given
    let temp = tempdir()?;
    let path = temp.path();
    // let path = Path::new("/tmp/test");
    given_file_with_contents(path, GLOB_FILTER_FILENAME, indoc!("
        + a/b*
        - a/**
    "))?;
    given_dir_with_modified_index(path, true)?;

    // When
    let result = run_update(temp.path(), true)?;

    // Then
    assert_eq!(status_code(&result), 0);

    let out = stdout(&result);
    assert!(match_regex(&out, r"(?m)^Updated:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Removed:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+2$"));

    assert!(out.contains("[*] f1.txt"));
    assert!(out.contains("[-] a/b/f3.txt"));
    assert!(out.contains("[-] c/large.txt"));

    Ok(())
}

fn run_init(base: &Path) -> io::Result<Output> {
    let path = base.to_string_lossy();
    Command::new(BINARY_PATH).
        arg("init").
        arg(path.as_ref()).
        output()
}

fn run_audit(base: &Path) -> io::Result<Output> {
    let path = base.to_string_lossy();
    Command::new(BINARY_PATH).
        arg("audit").
        arg(path.as_ref()).
        output()
}

fn run_update(base: &Path, cont: bool) -> Result<Output> {
    let path = base.to_string_lossy();
    let mut c = Command::new(BINARY_PATH).
        arg("update").
        arg(path.as_ref()).
        stdin(Stdio::piped()).
        stdout(Stdio::piped()).
        stderr(Stdio::piped()).
        spawn()?;

    if let Some(ref mut stdin) = c.stdin {
        let mut writer = BufWriter::new(stdin);

        if cont {
            writer.write_fmt(format_args!("Y\n"))?;
        } else {
            writer.write_fmt(format_args!("N\n"))?;
        }
    }

    c.wait_with_output().map_err(anyhow::Error::new)
}

fn given_dir_without_index(base: &Path) -> Result<()> {
    given_file_with_random_contents(base, "f1.txt", 4 * 1024)?;
    given_file_with_contents(base, "a/f2a.txt", "f2")?;
    given_file_with_contents(base, "a/f2b.txt", "f2")?;
    given_file_with_random_contents(base, "a/b/f3.txt", 123)?;
    given_file_with_random_contents(base, "c/f4.txt", 12345)?;
    given_file_with_random_contents(base, "c/large.txt", 10 * 1024 * 1024)?;
    Ok(())
}

fn given_dir_with_index(base: &Path) -> Result<()> {
    given_dir_without_index(base)?;
    let result = run_init(base)?;
    if status_code(&result) != 0 {
        bail!("init failed");
    }
    Ok(())
}

fn given_dir_with_modified_index(base: &Path, bitrot: bool) -> Result<()> {
    given_dir_with_index(base)?;

    given_file_with_random_contents(base, "a/new.txt", 10 * 1024)?; // New file
    replace_file_with_contents(base, "f1.txt", "new contents", false)?; // Updated file
    replace_file_with_contents(base, "a/f2b.txt", "f2", false)?; // Updated file, only mtime
    std::fs::remove_dir_all(base.join("a/b"))?; // Removed file
    std::fs::rename(base.join("c/large.txt"), base.join("a/large_new.txt"))?; // Moved file

    if bitrot {
        replace_file_with_contents(base, "a/f2a.txt", "new contents", true)?; // File with bitrot
    }

    Ok(())
}