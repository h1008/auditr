use std::path::Path;

use crate::index::{HASH_INDEX_NAME, META_INDEX_NAME};

pub mod globfilter;

pub trait PathFilter {
    fn matches(&self, p: &Path) -> bool;
}

pub struct DefaultPathFilter {
    excluded: [String; 2]
}

impl DefaultPathFilter {
    pub fn new(dir_name: &Path) -> DefaultPathFilter {
        let hash_idx_path = dir_name.join(Path::new(HASH_INDEX_NAME)).to_string_lossy().to_string();
        let meta_idx_path = dir_name.join(Path::new(META_INDEX_NAME)).to_string_lossy().to_string();
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

#[cfg(test)]
mod tests {
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
        test_meta_index_relative_path: (Path::new(META_INDEX_NAME), true),
        test_hash_index_relative_path: (Path::new(HASH_INDEX_NAME), true),
        test_meta_abs_path: (&Path::new("/some/path").join(META_INDEX_NAME), false),
        test_hash_abs_path: (&Path::new("/some/path").join(HASH_INDEX_NAME), false),
    }
}
