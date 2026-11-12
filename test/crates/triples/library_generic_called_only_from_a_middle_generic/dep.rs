//! A library whose generic functions its own crate never calls: their
//! exports take no drops (#1596).
pub struct Bag(pub Vec<u32>);

impl Extend<u32> for Bag {
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        for x in iter {
            self.0.push(x);
        }
    }
}

pub fn given<C: Extend<u32>, I: IntoIterator<Item = u32>>(mut c: C, items: I) -> C {
    c.extend(items);
    c
}

pub fn first<I: IntoIterator<Item = u32>>(items: I) -> Option<u32> {
    items.into_iter().next()
}
