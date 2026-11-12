//! A library none of whose values drops anything, with its own `Extend`
//! (#1591).
pub struct Bag(pub Vec<u32>);

impl Extend<u32> for Bag {
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        for x in iter {
            self.0.push(x);
        }
    }
}
