extern crate libc;
extern crate regex;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::os::raw::c_int;
use std::path::Path;
use std::process::Output;
use std::time;
use std::time::SystemTime;

use anyhow::Result;
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use rand::rngs::mock::StepRng;
use regex::Regex;

pub fn given_file_with_contents(base: &Path, path: &str, contents: &str) -> Result<()> {
    let file_path = base.join(path);

    std::fs::create_dir_all(file_path.parent().unwrap())?;
    std::fs::write(&file_path, contents)?;
    Ok(())
}

pub fn given_file_with_random_contents(base: &Path, path: &str, size: usize) -> Result<()> {
    let file_path = base.join(path);

    std::fs::create_dir_all(file_path.parent().unwrap())?;
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    for random in StepRng::new(0, 1)
        .sample_iter(&Alphanumeric)
        .take(size) {
        let b = [random as u8];
        writer.write(&b)?;
    }
    writer.flush()?;

    Ok(())
}

pub fn replace_file_with_contents(base: &Path, path: &str, contents: &str, keep_timestamps: bool) -> Result<()> {
    let file_path = base.join(path);

    let meta = std::fs::metadata(&file_path)?;

    std::fs::write(&file_path, contents)?;

    if keep_timestamps {
        set_unix_times(file_path.to_string_lossy().as_ref(), meta.accessed()?, meta.modified()?)?;
    }

    Ok(())
}

pub fn status_code(out: &Output) -> i32 {
    out.status.code().unwrap_or_default()
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

pub fn match_regex(str: &str, regexp: &str) -> bool {
    Regex::new(regexp).unwrap().is_match(str)
}

fn set_unix_times(path: &str, accessed: SystemTime, modified: SystemTime) -> Result<()> {
    use std::os::unix::io::AsRawFd;

    let f = File::open(path)?;
    let fd = f.as_raw_fd();

    let atime = accessed.duration_since(time::UNIX_EPOCH)?;
    let mtime = modified.duration_since(time::UNIX_EPOCH)?;
    let times = [
        libc::timespec {
            tv_sec: atime.as_secs() as libc::time_t,
            tv_nsec: atime.subsec_nanos() as libc::c_long,
        },
        libc::timespec {
            tv_sec: mtime.as_secs() as libc::time_t,
            tv_nsec: mtime.subsec_nanos() as libc::c_long,
        }];
    let rc = unsafe { libc::futimens(fd, times.as_ptr()) } as c_int;
    if rc == -1 {
        Err(anyhow::Error::new(std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

