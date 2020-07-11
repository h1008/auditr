# auditr

Auditr collects hashes and file system metadata of all files in a directory
tree. The collected data can be used at later point in time to detect changes
(like files added, removed, or updated).

## Features

- Recursively collect and store SHA256 hashes and file metadata (size, mtime)
- Detect added, removed, moved, and updated files
- Detect updated files without corresponding change of the file system's
  modified timestamp (bit rot)
- Store hashes in a format compatible to the sha256sum tool
- Incrementally update index files (only recompute hashes if file metadata has
  changed)

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

# Use sha256sum
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
| *      | File was updated                                                 |
| \>     | File was moved (i.e., different name but same contents)          |
| !      | File content changed but modification timestamp did not (bitrot) |

## Return Codes

| Return Code | Explanation                                     |
|-------------|-------------------------------------------------|
|           0 | Success                                         |
|           1 | Unrecoverable error                             |
|           2 | Audit failed, changes were detected (no bitrot) |
|           3 | Audit failed because bitrot was detected        |

## Limitations

- Tested on Linux only
