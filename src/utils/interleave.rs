pub struct Interleave<Iter: Iterator>(Vec<Iter>);

impl<Iter: Iterator> Iterator for Interleave<Iter> {
    type Item = Iter::Item;

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

pub trait IntoInterleave<Iter: Iterator> {
    fn interleave(self) -> Interleave<Iter>;
}

impl<Iters> IntoInterleave<Iters::Item> for Iters
where
    Iters: Iterator,
    Iters::Item: Iterator,
{
    fn interleave(self) -> Interleave<Iters::Item> {
        Interleave(self.into_iter().collect())
    }
}
