use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use glob::Pattern;
use lazy_static::lazy_static;

use crate::filter::PathFilter;
use crate::index::{HASH_INDEX_FILENAME, META_INDEX_FILENAME};

pub const GLOB_FILTER_FILENAME: &str = ".auditr-ignore";

#[derive(Clone,Debug)]
pub struct GlobRule {
    pattern: glob::Pattern,
    include: bool,
}

impl GlobRule {
    fn new(pattern: &str, include: bool) -> Result<GlobRule> {
        let pattern = Pattern::new(pattern)?;
        Ok(GlobRule {
            pattern,
            include,
        })
    }

    fn load_rules(file_name: &Path) -> Result<Vec<GlobRule>> {
        let file = File::open(file_name)?;
        let reader = BufReader::new(file);

        let line_contains_filter = |l: &String| !l.starts_with('#') && !l.trim().is_empty();
        let rules = reader.lines().
            filter(|line| line.as_ref().map(line_contains_filter).unwrap_or(true)).
            map(|line| GlobRule::try_from(line?.as_str())).
            collect::<Result<Vec<GlobRule>>>()?;

        let all_rules = DEFAULT_RULES.iter().
            cloned().
            chain(rules).
            collect();

        Ok(all_rules)
    }
}

impl TryFrom<&str> for GlobRule {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some(path) = value.strip_prefix('!') {
            GlobRule::new(path, true)
        } else {
            GlobRule::new(value, false)
        }
    }
}

impl Display for GlobRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let symbol = match self.include {
            true => "+",
            false => "-"
        };
        write!(f, "{} {}", symbol, self.pattern.as_str())
    }
}

#[derive(Debug)]
pub struct GlobPathFilter<'a> {
    rules: Vec<GlobRule>,
    root: &'a Path,
    include_by_default: bool,
}

lazy_static! {
    static ref DEFAULT_RULES: Vec<GlobRule> = vec![
        GlobRule::new(HASH_INDEX_FILENAME, false).unwrap(),
        GlobRule::new(META_INDEX_FILENAME, false).unwrap(),
        GlobRule::new(GLOB_FILTER_FILENAME, false).unwrap(),
    ];
}

impl GlobPathFilter<'_> {
    pub fn new(root: &Path, rules: Vec<GlobRule>, include_by_default: bool) -> Result<GlobPathFilter> {
        Ok(GlobPathFilter {
            rules,
            root,
            include_by_default,
        })
    }

    pub fn default(root: &Path) -> Result<GlobPathFilter> {
        GlobPathFilter::new(root, DEFAULT_RULES.clone(), true)
    }

    pub fn load_from_path(path: &Path, include_by_default: bool) -> Result<GlobPathFilter> {
        let rules = GlobRule::load_rules(&path.join(GLOB_FILTER_FILENAME))?;
        GlobPathFilter::new(path, rules, include_by_default)
    }
}

impl PathFilter for GlobPathFilter<'_> {
    fn matches(&self, p: &Path) -> bool {
        if let Ok(rel_path) = p.strip_prefix(self.root) {
            let rule = self.rules.iter().find(|i| i.pattern.matches_path(rel_path));
            return match rule {
                Some(rule) => rule.include,
                None => self.include_by_default
            };
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::*;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_matches_no_rules_default_include() -> Result<()> {
        let patterns = vec![];
        let filter = GlobPathFilter::new(Path::new("/some/path"), patterns, true)?;

        assert_eq!(filter.matches(Path::new("/some/path/test.txt")), true);

        Ok(())
    }

    #[test]
    fn test_matches_no_rules_default_exclude() -> Result<()> {
        let patterns = vec![];
        let filter = GlobPathFilter::new(Path::new("/some/path"), patterns, false)?;

        assert_eq!(filter.matches(Path::new("/some/path/test.txt")), false);

        Ok(())
    }

    #[test]
    fn test_matches_different_root() -> Result<()> {
        let patterns = vec![];
        let filter = GlobPathFilter::new(Path::new("/some/path"), patterns, true)?;

        assert_eq!(filter.matches(Path::new("/some/other/path/test.txt")), false);

        Ok(())
    }

    #[test]
    fn test_matches_use_first_matching_rule() -> Result<()> {
        let patterns = vec![
            GlobRule::new("**/a.txt", true)?,
            GlobRule::new("a", false)?,
            GlobRule::new("b/*.txt", true)?,
            GlobRule::new("**/b.txt", false)?,
        ];
        let filter = GlobPathFilter::new(Path::new("/some/path"), patterns, false)?;

        assert_eq!(filter.matches(Path::new("/some/path/a.txt")), true);
        assert_eq!(filter.matches(Path::new("/some/path/a/a.txt")), true);
        assert_eq!(filter.matches(Path::new("/some/path/a/b.txt")), false);
        assert_eq!(filter.matches(Path::new("/some/path/b/b.txt")), true);
        assert_eq!(filter.matches(Path::new("/some/path/b/c.txt")), true);
        assert_eq!(filter.matches(Path::new("/some/path/other.txt")), false);

        Ok(())
    }

    macro_rules! try_from_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() -> Result<()> {
                let (input, expected_include, expected_path) = $value;

                let rule = GlobRule::try_from(input)?;

                assert_eq!(rule.include, expected_include);
                assert_eq!(rule.pattern.as_str(), expected_path);

                Ok(())
            }
        )*
        }
    }

    try_from_tests! {
        test_try_from_include: ("!**/test.txt", true, "**/test.txt"),
        test_try_from_exclude: ("**/test.txt", false, "**/test.txt"),
    }

    #[test]
    fn test_display() -> Result<()> {
        let rule = GlobRule::new("**/test.txt", true)?;
        assert_eq!(format!("{}", rule), "+ **/test.txt");

        let rule = GlobRule::new("file.txt", false)?;
        assert_eq!(format!("{}", rule), "- file.txt");

        Ok(())
    }

    #[test]
    fn test_default_filter() -> Result<()> {
        let filter = GlobPathFilter::default(Path::new("/some/path"))?;
        assert_eq!(filter.matches(Path::new("/some/path/test.txt")), true);
        assert_eq!(filter.matches(Path::new("/some/path/.auditr-meta")), false);
        assert_eq!(filter.matches(Path::new("/some/path/.auditr-sha256")), false);
        assert_eq!(filter.matches(Path::new("/some/path/dir/.auditr-meta")), true);
        assert_eq!(filter.matches(Path::new("/some/path/dir/.auditr-sha256")), true);

        Ok(())
    }

    #[test]
    fn test_load_rules() -> Result<()> {
        let temp = tempdir()?;

        let path = temp.path().join(".auditr-ignore");
        let rules_file = indoc!("
            # some comment
            !some/dir/file.txt

            # another comment
            some/dir/*
        ");
        fs::write(path.as_path(), rules_file)?;

        let rules = GlobRule::load_rules(path.as_path())?;

        assert_eq!(rules.len(), 5);
        assert_eq!(rules[0].pattern.as_str(), HASH_INDEX_FILENAME);
        assert_eq!(rules[0].include, false);
        assert_eq!(rules[1].pattern.as_str(), META_INDEX_FILENAME);
        assert_eq!(rules[1].include, false);
        assert_eq!(rules[2].pattern.as_str(), GLOB_FILTER_FILENAME);
        assert_eq!(rules[2].include, false);
        assert_eq!(rules[3].pattern.as_str(), "some/dir/file.txt");
        assert_eq!(rules[3].include, true);
        assert_eq!(rules[4].pattern.as_str(), "some/dir/*");
        assert_eq!(rules[4].include, false);

        Ok(())
    }

    #[test]
    fn test_load_from_path() -> Result<()> {
        // Given
        let temp = tempdir()?;

        let path = temp.path().join(".auditr-ignore");
        let rules_file = indoc!("
            !some/dir/file.txt
            some/dir/*
        ");
        fs::write(path.as_path(), rules_file)?;

        // When
        let filter = GlobPathFilter::load_from_path(temp.path(), true)?;

        // Then
        assert_eq!(filter.rules.len(), 5);

        Ok(())
    }
}
