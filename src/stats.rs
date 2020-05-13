use std::iter::FromIterator;

use crate::diff;
use crate::diff::Event;
use crate::index::Entry;
use std::collections::HashMap;
use std::path::PathBuf;

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
        self.added.clone().into_iter().
            chain(self.unchanged.clone().into_iter()).
            chain(self.updated.clone().into_iter())
    }

    fn compute_moved(&mut self) {
        let mut removed = self.removed.iter().
            map(|e| (e.hash.clone(), e.clone())).
            collect::<HashMap<String, &'a Entry>>();

        let mut added = Vec::new();
        for a in &self.added {
            match removed.remove_entry(&a.hash) {
                Some((_, val)) => {
                    self.moved.insert(val.path.clone(), a);
                },
                None => {
                    added.push(a.clone());
                }
            }
        }

        self.added = added;
        self.removed = removed.into_iter().map(|x| x.1).collect();
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
                diff::Event::UNCHANGED { old: _, new } => {
                    stats.unchanged.push(new);
                    stats.total += 1;
                }
            }
        }

        stats.compute_moved();

        stats
    }
}
