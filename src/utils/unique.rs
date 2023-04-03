use std::hash::Hash;

use fxhash::FxHashSet;

pub struct Unique<Iter>(Iter, FxHashSet<Iter::Item>)
where
    Iter: Iterator,
    Iter::Item: Hash + Eq;

impl<Iter> Unique<Iter>
where
    Iter: Iterator,
    Iter::Item: Hash + Eq,
{
    fn new(iter: Iter) -> Self {
        Self(iter, FxHashSet::default())
    }
}

impl<Iter> Iterator for Unique<Iter>
where
    Iter: Iterator,
    Iter::Item: Hash + Eq + Clone,
{
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        for item in self.0.by_ref() {
            if !self.1.insert(item.clone()) {
                // the item was already in the hashset
                continue;
            }

            return Some(item);
        }

        None
    }
}

pub trait IntoUnique<Iter>
where
    Iter: Iterator,
    Iter::Item: Hash + Eq,
{
    fn unique(self) -> Unique<Iter>;
}

impl<Iter> IntoUnique<Iter> for Iter
where
    Iter: Iterator,
    Iter::Item: Hash + Eq,
{
    fn unique(self) -> Unique<Iter> {
        Unique::new(self)
    }
}
