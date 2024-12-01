use std::collections::HashMap;
use std::iter::FromIterator;
use std::path::PathBuf;

use crate::diff;
use crate::diff::Event;
use crate::entry::Entry;

pub struct Stats<'a> {
    pub added: Vec<&'a Entry>,
    pub removed: Vec<&'a Entry>,
    pub updated: Vec<&'a Entry>,
    pub updated_bitrot: Vec<&'a Entry>,
    pub moved: HashMap<PathBuf, &'a Entry>,
    pub unchanged: Vec<&'a Entry>,
    pub total: u64,
}

impl<'a> Stats<'a> {
    pub fn modified(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty() ||
            !self.updated.is_empty() || !self.updated_bitrot.is_empty() || !self.moved.is_empty()
    }

    pub fn iter_new(&self) -> impl Iterator<Item=&'a Entry> {
        let moved_entries: Vec<&'a Entry> = self.moved.values().copied().collect();
        self.added.clone().into_iter().
            chain(self.unchanged.clone().into_iter()).
            chain(self.updated.clone().into_iter()).
            chain(self.updated_bitrot.clone().into_iter()).
            chain(moved_entries)
    }

    fn compute_moved(&mut self) {
        let mut removed = self.removed.iter().
            map(|&e| (e.hash.clone(), e)).
            collect::<HashMap<String, &'a Entry>>();

        let mut added = Vec::new();
        for &a in &self.added {
            match removed.remove_entry(&a.hash) {
                Some((_, val)) => {
                    self.moved.insert(val.path.clone(), a);
                }
                None => {
                    added.push(a);
                }
            }
        }

        let mut removed = Vec::new();
        for &r in &self.removed {
            if self.moved.get(&r.path).is_none() {
                removed.push(r);
            }
        }

        self.added = added;
        self.removed = removed
    }
}

impl<'a> FromIterator<Event<'a, Entry>> for Stats<'a> {
    fn from_iter<T: IntoIterator<Item=Event<'a, Entry>>>(iter: T) -> Self {
        let mut stats = Stats {
            added: Vec::new(),
            removed: Vec::new(),
            updated: Vec::new(),
            updated_bitrot: Vec::new(),
            unchanged: Vec::new(),
            moved: HashMap::new(),
            total: 0,
        };

        for event in iter {
            match event {
                diff::Event::ADDED { new } => {
                    stats.added.push(new);
                    stats.total += 1;
                }
                diff::Event::REMOVED { old } => {
                    stats.removed.push(old);
                }
                diff::Event::UPDATED { old, new }
                if old.modified == new.modified && !Entry::compare_hash(old, new) => {
                    stats.updated_bitrot.push(new);
                    stats.total += 1;
                }
                diff::Event::UPDATED { old: _, new } => {
                    stats.updated.push(new);
                    stats.total += 1;
                }
                diff::Event::UNCHANGED { old, new: _ } => {
                    stats.unchanged.push(old);
                    stats.total += 1;
                }
            }
        }

        stats.compute_moved();

        stats
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use diff::Event;

    use super::*;

    fn given_entry(name: &str) -> Entry {
        let hash = format!("{} hash", name);
        return Entry {
            path: PathBuf::from(name),
            norm_path: name.to_owned(),
            hash,
            len: 123,
            modified: 123,
        };
    }

    #[test]
    fn test_modified_empty() -> Result<()> {
        // When
        let stats = Stats {
            added: vec![],
            removed: vec![],
            updated: vec![],
            updated_bitrot: vec![],
            moved: Default::default(),
            unchanged: vec![],
            total: 0,
        };

        // Then
        assert_eq!(stats.modified(), false);

        Ok(())
    }

    #[test]
    fn test_modified_unchanged() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        // When
        let stats = Stats {
            added: vec![],
            removed: vec![],
            updated: vec![],
            updated_bitrot: vec![],
            moved: Default::default(),
            unchanged: vec![&entry],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), false);

        Ok(())
    }

    #[test]
    fn test_modified_added() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        // When
        let stats = Stats {
            added: vec![&entry],
            removed: vec![],
            updated: vec![],
            updated_bitrot: vec![],
            moved: Default::default(),
            unchanged: vec![],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), true);

        Ok(())
    }

    #[test]
    fn test_modified_removed() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        // When
        let stats = Stats {
            added: vec![],
            removed: vec![&entry],
            updated: vec![],
            updated_bitrot: vec![],
            moved: Default::default(),
            unchanged: vec![],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), true);

        Ok(())
    }

    #[test]
    fn test_modified_updated() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        // When
        let stats = Stats {
            added: vec![],
            removed: vec![],
            updated: vec![&entry],
            updated_bitrot: vec![],
            moved: Default::default(),
            unchanged: vec![],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), true);

        Ok(())
    }

    #[test]
    fn test_modified_bitrot() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        // When
        let stats = Stats {
            added: vec![],
            removed: vec![],
            updated: vec![],
            updated_bitrot: vec![&entry],
            moved: Default::default(),
            unchanged: vec![],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), true);

        Ok(())
    }

    #[test]
    fn test_modified_moved() -> Result<()> {
        // Given
        let entry = given_entry("file.txt");

        let mut moved_files: HashMap<PathBuf, &Entry> = HashMap::new();
        moved_files.insert(PathBuf::from("old-file.txt"), &entry);

        // When
        let stats = Stats {
            added: vec![],
            removed: vec![],
            updated: vec![],
            updated_bitrot: vec![],
            moved: moved_files,
            unchanged: vec![],
            total: 1,
        };

        // Then
        assert_eq!(stats.modified(), true);

        Ok(())
    }

    #[test]
    fn test_iter_new() -> Result<()> {
        // Given
        let added_entry = given_entry("added.txt");
        let removed_entry = given_entry("removed.txt");
        let updated_entry = given_entry("updated.txt");
        let bitrot_entry = given_entry("bitrot.txt");
        let moved_entry = given_entry("moved.txt");
        let unchanged_entry = given_entry("unchanged.txt");

        let mut moved_files: HashMap<PathBuf, &Entry> = HashMap::new();
        moved_files.insert(PathBuf::from("old-file.txt"), &moved_entry);

        // When
        let stats = Stats {
            added: vec![&added_entry],
            removed: vec![&removed_entry],
            updated: vec![&updated_entry],
            updated_bitrot: vec![&bitrot_entry],
            moved: moved_files,
            unchanged: vec![&unchanged_entry],
            total: 6,
        };

        let entries: Vec<&Entry> = stats.iter_new().collect();

        // Then
        assert_eq!(entries, vec![&added_entry, &unchanged_entry, &updated_entry, &bitrot_entry, &moved_entry]);

        Ok(())
    }

    #[test]
    fn test_from_iter_added() -> Result<()> {
        // Given
        let added_entry_1 = given_entry("new1.txt");
        let added_entry_2 = given_entry("new2.txt");

        let events = vec![
            Event::ADDED {
                new: &added_entry_1
            },
            Event::ADDED {
                new: &added_entry_2
            },
        ];

        // When
        let stats = Stats::from_iter(events);

        // Then
        assert_eq!(stats.total, 2);
        assert_eq!(stats.added, vec![&added_entry_1, &added_entry_2]);

        Ok(())
    }

    #[test]
    fn test_from_iter_removed() -> Result<()> {
        // Given
        let removed_entry_1 = given_entry("removed1.txt");
        let removed_entry_2 = given_entry("removed2.txt");

        let events = vec![
            Event::REMOVED { old: &removed_entry_1 },
            Event::REMOVED { old: &removed_entry_2 },
        ];

        // When
        let stats = Stats::from_iter(events);

        // Then
        assert_eq!(stats.total, 0);
        assert_eq!(stats.removed, vec![&removed_entry_1, &removed_entry_2]);

        Ok(())
    }

    #[test]
    fn test_from_iter_updated() -> Result<()> {
        // Given
        let updated_entry_old = given_entry("updated.txt");
        let updated_entry_new = Entry {
            path: PathBuf::from("updated.txt"),
            norm_path: String::from("updated.txt"),
            hash: String::from("updated.txt new hash"),
            len: 456,
            modified: 234,
        };

        let updated_entry_with_bitrot_old = given_entry("bitrot.txt");
        let updated_entry_with_bitrot_new = Entry {
            path: PathBuf::from("bitrot.txt"),
            norm_path: String::from("bitrot.txt"),
            hash: String::from("bitrot new hash"),
            len: 123,
            modified: 123,
        };

        let events = vec![
            Event::UPDATED { old: &updated_entry_old, new: &updated_entry_new },
            Event::UPDATED { old: &updated_entry_with_bitrot_old, new: &updated_entry_with_bitrot_new },
        ];

        // When
        let stats = Stats::from_iter(events);

        // Then
        assert_eq!(stats.total, 2);
        assert_eq!(stats.updated, vec![&updated_entry_new]);
        assert_eq!(stats.updated_bitrot, vec![&updated_entry_with_bitrot_new]);

        Ok(())
    }

    #[test]
    fn test_from_iter_unchanged() -> Result<()> {
        // Given
        let unchanged_entry_1 = given_entry("unchanged_1.txt");
        let unchanged_entry_2 = given_entry("unchanged_2.txt");

        let events = vec![
            Event::UNCHANGED {
                old: &unchanged_entry_1,
                new: &unchanged_entry_1,
            },
            Event::UNCHANGED {
                old: &unchanged_entry_2,
                new: &unchanged_entry_2,
            },
        ];

        // When
        let stats = Stats::from_iter(events);

        // Then
        assert_eq!(stats.total, 2);
        assert_eq!(stats.unchanged, vec![&unchanged_entry_1, &unchanged_entry_2]);

        Ok(())
    }

    #[test]
    fn test_from_iter_moved() -> Result<()> {
        // When
        let moved_entry_1_from = Entry {
            path: PathBuf::from("moved_1_from.txt"),
            norm_path: String::from("moved_1_from.txt"),
            hash: String::from("moved file 1 hash"),
            len: 123,
            modified: 123,
        };
        let moved_entry_1a_to = Entry {
            path: PathBuf::from("moved_1a_to.txt"),
            norm_path: String::from("moved_1a_to.txt"),
            hash: String::from("moved file 1 hash"),
            len: 123,
            modified: 123,
        };
        let moved_entry_1b_to = Entry {
            path: PathBuf::from("moved_1b_to.txt"),
            norm_path: String::from("moved_1b_to.txt"),
            hash: String::from("moved file 1 hash"),
            len: 123,
            modified: 123,
        };
        let moved_entry_2a_from = Entry {
            path: PathBuf::from("moved_2a_from.txt"),
            norm_path: String::from("moved_2a_from.txt"),
            hash: String::from("moved file 2 hash"),
            len: 123,
            modified: 123,
        };
        let moved_entry_2b_from = Entry {
            path: PathBuf::from("moved_2b_from.txt"),
            norm_path: String::from("moved_2b_from.txt"),
            hash: String::from("moved file 2 hash"),
            len: 123,
            modified: 123,
        };
        let moved_entry_2_to = Entry {
            path: PathBuf::from("moved_2_to.txt"),
            norm_path: "mmoved_2_to.txt".to_owned(),
            hash: String::from("moved file 2 hash"),
            len: 123,
            modified: 123,
        };

        let events = vec![
            Event::REMOVED { old: &moved_entry_2b_from },
            Event::REMOVED { old: &moved_entry_2a_from },
            Event::ADDED {
                new: &moved_entry_1a_to
            },
            Event::ADDED {
                new: &moved_entry_1b_to
            },
            Event::ADDED {
                new: &moved_entry_2_to
            },
            Event::REMOVED { old: &moved_entry_1_from },
        ];

        // When
        let stats = Stats::from_iter(events);

        // Then
        assert_eq!(stats.total, 3);
        assert_eq!(stats.moved.len(), 2);
        assert_eq!(stats.moved.get(moved_entry_1_from.path.as_path()), Some(&&moved_entry_1a_to));
        assert_eq!(stats.moved.get(moved_entry_2a_from.path.as_path()), Some(&&moved_entry_2_to));
        assert_eq!(stats.added, vec![&moved_entry_1b_to]);
        assert_eq!(stats.removed, vec![&moved_entry_2b_from]);

        Ok(())
    }
}