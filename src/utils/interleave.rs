pub struct Interleave<Iter: Iterator<Item = Item>, Item>(Vec<Iter>);

impl<Iter: Iterator<Item = Item>, Item> Iterator for Interleave<Iter, Item> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(item) = self.0.last_mut()?.next() else {
                self.0.pop();
                continue;
            };
            self.0.rotate_right(1);
            return Some(item);
        }
    }
}

pub trait IntoInterleave<Iter: Iterator<Item = Item>, Item> {
    fn interleave(self) -> Interleave<Iter, Item>;
}

impl<Iter: Iterator<Item = Item>, Item, Outer: IntoIterator<Item = Iter>> IntoInterleave<Iter, Item>
    for Outer
{
    fn interleave(self) -> Interleave<Iter, Item> {
        Interleave(self.into_iter().collect())
    }
}
