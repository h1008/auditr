use std::io;
use std::path::Path;
use std::process::{Command, Output};

use anyhow::Result;
use tempfile::tempdir;

use common::*;

mod common;

const BINARY_PATH: &str = "target/debug/hello-rust";

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

fn given_some_files(base: &Path) -> Result<()> {
    given_file_with_random_contents(base, "f1.txt", 4 * 1024)?;
    given_file_with_contents(base, "a/f2a.txt", "f2")?;
    given_file_with_contents(base, "a/f2b.txt", "f2")?;
    given_file_with_random_contents(base, "a/b/f3.txt", 123)?;
    given_file_with_random_contents(base, "c/f4.txt", 12345)?;
    given_file_with_random_contents(base, "c/large.txt", 10 * 1024 * 1024)?;
    Ok(())
}

#[test]
fn test_init_audit() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_some_files(temp.path())?;

    // When
    let result = run_init(temp.path())?;

    // Then
    assert_eq!(status_code(&result), 0);
    assert!(stderr(&result).contains(&format!("Initializing indices in '{}'", temp.path().display())));
    assert!(stderr(&result).contains("Done."));

    // When
    let result = run_audit(temp.path())?;

    // Then
    let out = stdout(&result);
    assert_eq!(status_code(&result), 0);
    assert!(match_regex(&out, r"(?m)^New:\s+0$"));
    assert!(match_regex(&out, r"(?m)^Updated:\s+0$"));
    assert!(match_regex(&out, r"(?m)^Updated \(bitrot\):\s+0$"));
    assert!(match_regex(&out, r"(?m)^Removed:\s+0$"));
    assert!(match_regex(&out, r"(?m)^Moved:\s+0$"));
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+6$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+6$"));
    assert!(out.contains("Audit successful"));

    Ok(())
}

#[test]
fn test_init_modify_audit() -> Result<()> {
    // Given
    let temp = tempdir()?;
    given_some_files(temp.path())?;

    let result = run_init(temp.path())?;
    assert_eq!(status_code(&result), 0);

    given_file_with_random_contents(temp.path(), "a/new.txt", 10 * 1024)?; // New file
    replace_file_with_contents(temp.path(), "f1.txt", "new contents", false)?; // Updated file
    replace_file_with_contents(temp.path(), "a/f2a.txt", "new contents", true)?; // File with bitrot
    std::fs::remove_dir_all(temp.path().join("a/b"))?; // Removed file
    std::fs::rename(temp.path().join("c/large.txt"), temp.path().join("a/large_new.txt"))?; // Moved file

    // When
    let result = run_audit(temp.path())?;

    // Then
    let out = stdout(&result);

    assert_eq!(status_code(&result), 1);
    assert!(match_regex(&out, r"(?m)^New:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Updated:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Updated \(bitrot\):\s+1$"));
    assert!(match_regex(&out, r"(?m)^Removed:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Moved:\s+1$"));
    assert!(match_regex(&out, r"(?m)^Unchanged:\s+2$"));
    assert!(match_regex(&out, r"(?m)^Total:\s+6$"));

    assert!(out.contains("[+] a/new.txt"));
    assert!(out.contains("[*] f1.txt"));
    assert!(out.contains("[!] a/f2a.txt"));
    assert!(out.contains("[-] a/b/f3.txt"));
    assert!(out.contains("[>] a/large_new.txt (from c/large.txt)"));

    Ok(())
}

// 3) Init - Modify - Update - Audit

// Audit, Modify without init
// Large file
