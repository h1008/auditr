use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::fmt;

#[derive(Debug,Eq,PartialEq)]
pub enum Event<'a, T> {
    ADDED { new: &'a T },
    REMOVED { old: &'a T },
    UPDATED { old: &'a T, new: &'a T },
    UNCHANGED { old: &'a T, new: &'a T },
}

impl<'a, T> Display for Event<'a, T> where
    T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

mod tests {
    #[cfg(test)]
    use super::*;

    #[test]
    fn test_diff_empty() {
        let a: Vec<&i32> = vec![];
        let b: Vec<&i32> = vec![];

        let result = diff_iter(a.into_iter(), b.into_iter(), |_, _| true).
            collect::<Vec<Event<i32>>>();

        assert!(result.is_empty());
    }

    #[test]
    fn test_diff_unchanged() {
        let a = vec![&1, &2, &3];
        let b = vec![&1, &2, &3];

        let result = diff_iter(a.into_iter(), b.into_iter(), |_, _| true).
            collect::<Vec<Event<i32>>>();

        let expected: Vec<Event<i32>> = vec![
            Event::UNCHANGED {old: &1, new: &1},
            Event::UNCHANGED {old: &2, new: &2},
            Event::UNCHANGED {old: &3, new: &3},
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_diff_updated() {
        let a = vec![&1, &2, &3];
        let b = vec![&1, &2, &3];

        let result = diff_iter(a.into_iter(), b.into_iter(), |_, _| false).
            collect::<Vec<Event<i32>>>();

        let expected: Vec<Event<i32>> = vec![
            Event::UPDATED {old: &1, new: &1},
            Event::UPDATED {old: &2, new: &2},
            Event::UPDATED {old: &3, new: &3},
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_diff_added() {
        let a: Vec<&i32> = vec![];
        let b = vec![&1, &2, &3];

        let result = diff_iter(a.into_iter(), b.into_iter(), |_, _| false).
            collect::<Vec<Event<i32>>>();

        let expected: Vec<Event<i32>> = vec![
            Event::ADDED {new: &1},
            Event::ADDED {new: &2},
            Event::ADDED {new: &3},
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_diff_removed() {
        let a = vec![&1, &2, &3];
        let b: Vec<&i32> = vec![];

        let result = diff_iter(a.into_iter(), b.into_iter(), |_, _| false).
            collect::<Vec<Event<i32>>>();

        let expected: Vec<Event<i32>> = vec![
            Event::REMOVED {old: &1},
            Event::REMOVED {old: &2},
            Event::REMOVED {old: &3},
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_diff_complex() {
        let a = vec![&1, &2, &3, &6];
        let b = vec![&1, &4, &5, &6];

        let result = diff_iter(a.into_iter(), b.into_iter(), |a, b| *a != 6 || *b != 6).
            collect::<Vec<Event<i32>>>();

        let expected: Vec<Event<i32>> = vec![
            Event::UNCHANGED {old: &1, new: &1},
            Event::REMOVED {old: &2},
            Event::REMOVED {old: &3},
            Event::ADDED {new: &4},
            Event::ADDED {new: &5},
            Event::UPDATED {old: &6, new: &6},
        ];
        assert_eq!(expected, result);
    }
}