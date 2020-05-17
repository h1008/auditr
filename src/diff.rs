use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum Event<'a, T> {
    ADDED { new: &'a T },
    REMOVED { old: &'a T },
    UPDATED { old: &'a T, new: &'a T },
    UNCHANGED { old: &'a T, new: &'a T },
}

impl<'a, T> Display for Event<'a, T> where
    T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Event::ADDED { new } => write!(f, "+ {}", new),
            Event::REMOVED { old } => write!(f, "- {}", old),
            Event::UPDATED { old: _, new } => write!(f, "* {}", new),
            Event::UNCHANGED { old: _, new } => write!(f, "  {}", new),
        }
    }
}

pub fn diff_iter<'a, I, F, T>(old_iter: I, new_iter: I, equal: F) -> DiffIter<'a, I, F, T> where
    I: Iterator<Item=&'a T>,
    F: Fn(&'a T, &'a T) -> bool {
    let mut o = old_iter;
    let mut n = new_iter;
    DiffIter {
        old_val: o.next(),
        new_val: n.next(),
        old_iter: o,
        new_iter: n,
        equal,
    }
}

pub struct DiffIter<'a, I, F, T> where
    I: Iterator<Item=&'a T>,
    F: Fn(&'a T, &'a T) -> bool {
    old_iter: I,
    new_iter: I,
    old_val: Option<&'a T>,
    new_val: Option<&'a T>,
    equal: F,
}

impl<'a, I, F, T> DiffIter<'a, I, F, T> where
    I: Iterator<Item=&'a T>,
    F: Fn(&'a T, &'a T) -> bool {
    fn added_key(&mut self, new: &'a T) -> Option<Event<'a, T>> {
        self.new_val = self.new_iter.next();
        Some(Event::ADDED { new })
    }

    fn removed_key(&mut self, old: &'a T) -> Option<Event<'a, T>> {
        self.old_val = self.old_iter.next();
        Some(Event::REMOVED { old })
    }

    fn same_key(&mut self, old: &'a T, new: &'a T) -> Option<Event<'a, T>> {
        self.old_val = self.old_iter.next();
        self.new_val = self.new_iter.next();

        if (self.equal)(old, new) {
            Some(Event::UNCHANGED { old, new })
        } else {
            Some(Event::UPDATED { old, new })
        }
    }
}

impl<'a, I, F, T> Iterator for DiffIter<'a, I, F, T> where
    I: Iterator<Item=&'a T>,
    F: Fn(&'a T, &'a T) -> bool,
    T: Ord {
    type Item = Event<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.old_val, self.new_val) {
            (Some(old), Some(new)) => {
                match old.cmp(new) {
                    Ordering::Equal => self.same_key(old, new),
                    Ordering::Less => self.removed_key(old),
                    Ordering::Greater => self.added_key(new)
                }
            }
            (Some(old), None) => self.removed_key(old),
            (None, Some(new)) => self.added_key(new),
            (None, None) => None
        }
    }
}