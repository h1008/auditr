use std::path::Path;

use anyhow::Result;

use crate::filter::globfilter::{GLOB_FILTER_FILENAME, GlobPathFilter};
use crate::index::{HASH_INDEX_FILENAME, META_INDEX_FILENAME};

pub mod globfilter;

pub trait PathFilter {
    fn matches(&self, p: &Path) -> bool;
}

pub struct DefaultPathFilter {
    excluded: [String; 2]
}

impl DefaultPathFilter {
    pub fn new(dir_name: &Path) -> DefaultPathFilter {
        let hash_idx_path = dir_name.join(Path::new(HASH_INDEX_FILENAME)).to_string_lossy().to_string();
        let meta_idx_path = dir_name.join(Path::new(META_INDEX_FILENAME)).to_string_lossy().to_string();
        DefaultPathFilter {
            excluded: [hash_idx_path, meta_idx_path]
        }
    }
}

impl PathFilter for DefaultPathFilter {
    fn matches(&self, p: &Path) -> bool {
        !self.excluded.contains(&p.to_string_lossy().to_string())
    }
}

pub fn load_filter<'a>(path: &'a Path) -> Result<Box<dyn PathFilter + 'a>>{
    if path.join(GLOB_FILTER_FILENAME).exists() {
        let filter = GlobPathFilter::load_from_path(path, true)?;
        return Ok(Box::new(filter));
    }

    Ok(Box::new(DefaultPathFilter::new(path)))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use indoc::indoc;

    use super::*;

    macro_rules! default_filter_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                let filter = DefaultPathFilter::new(Path::new("/some/path"));
                assert_eq!(filter.matches(input), expected);
            }
        )*
        }
    }

    default_filter_tests! {
        test_full_path: (Path::new("/some/path/a/test.txt"), true),
        test_relative_path: (Path::new("a/test.txt"), true),
        test_meta_index_relative_path: (Path::new(META_INDEX_FILENAME), true),
        test_hash_index_relative_path: (Path::new(HASH_INDEX_FILENAME), true),
        test_meta_abs_path: (&Path::new("/some/path").join(META_INDEX_FILENAME), false),
        test_hash_abs_path: (&Path::new("/some/path").join(HASH_INDEX_FILENAME), false),
    }

    #[test]
    fn test_load_filter_existing_ignorefile() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let path = temp.path().join(GLOB_FILTER_FILENAME);
        let rules_file = indoc!("
            !some/dir/file.txt
            some/dir/**
        ");
        fs::write(path.as_path(), rules_file)?;

        // When
        let filter = load_filter(temp.path())?;

        // Then
        assert_eq!(filter.matches(&temp.path().join("some/dir/file.txt")), true);
        assert_eq!(filter.matches(&temp.path().join("some/dir/other.txt")), false);
        assert_eq!(filter.matches(&temp.path().join("yet/another.txt")), true);
        assert_eq!(filter.matches(&temp.path().join(META_INDEX_FILENAME)), false);
        assert_eq!(filter.matches(&temp.path().join(HASH_INDEX_FILENAME)), false);

        Ok(())
    }

    #[test]
    fn test_load_filter_no_ignorefile() -> Result<()> {
        // Given
        let temp = tempdir()?;

        // When
        let filter = load_filter(temp.path())?;

        // Then
        assert_eq!(filter.matches(&temp.path().join("some/dir/other.txt")), true);
        assert_eq!(filter.matches(&temp.path().join(META_INDEX_FILENAME)), false);
        assert_eq!(filter.matches(&temp.path().join(HASH_INDEX_FILENAME)), false);

        Ok(())
    }
}
