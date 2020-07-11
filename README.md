# auditr

Auditr collects hashes and file system metadata of all files in a directory
tree. The collected data can be used at later point in time to detect changes
(like files that have been added, removed, or updated).

## Features

- Recursively collect and store SHA256 hashes and file metadata (size, mtime)
- Detect added, removed, moved, and updated files
- Detect updated files without corresponding change of the file system's
  modified timestamp (bitrot)
- Store hashes in a format compatible to the sha256sum tool
- Incrementally update index files (only recompute hashes if file metadata has
  changed)
- Exclude directories and files based on glob patterns

## Usage

```shell script
# Initialize the index
auditr init ~/Downloads/

# Do some changes
touch ~/Downloads/test.txt

# Compare files with index
auditr audit ~/Downloads

# Update the index
auditr update ~/Downloads

# Alternatively, audit and update in one step
auditr audit --update ~/Downloads

# Use sha256sum to verify the files manually
cd ~/Downloads
sha256sum -c .auditr-sha256
```

## Output

Auditr uses the following symbols to indicate detected differences between the
directory tree and the recorded index:

| Symbol | Explanation                                                      |
|--------|------------------------------------------------------------------|
| +      | File was added                                                   |
| -      | File was removed                                                 |
| *      | File was updated (contents and/or metadata                       |
| \>     | File was moved (i.e., different name but same contents)          |
| !      | File content changed but modification timestamp did not (bitrot) |

## Return Codes

| Return Code | Explanation                                     |
|-------------|-------------------------------------------------|
|           0 | Success                                         |
|           1 | Unrecoverable error                             |
|           2 | Audit failed, changes were detected (no bitrot) |
|           3 | Audit failed because bitrot was detected        |

## Ignore file

You can specify exclusion rules by creating a file named `.auditr-ignore` in the
target directory. Auditr matches all directory entries against all rules in this
file from top to bottom. The first matching rule determines the result. There
are two types of rules:

- Exclusion rules consist of a glob pattern. Auditr ignores all files and
  directories matching the pattern.
- Inclusion rules consist of an exclamation mark '!' followed by a glob pattern.
  Auditr will process all files and directories that match the pattern.

For a description of the glob pattern syntax see
[glob](https://docs.rs/glob/0.3.0/glob/struct.Pattern.html). Patterns must match
the complete path (relative to the target directory). Lines starting with '#'
will not be processed (comments).
 
Example:

```
# Include some/dir/file.txt
!some/dir/file.txt

# Exclude everything else in some/dir
some/dir/*
```

## Limitations

- Tested on Linux only
