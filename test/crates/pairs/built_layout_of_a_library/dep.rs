//! A library's generic function collecting into a `Vec` of a type built of
//! its type parameters, `Held<<H::Item as IntoIterator>::IntoIter>`, as
//! itertools' `multi_cartesian_product` does: whether std collects it in
//! place is its consumer's to say, of its source and that type's layout,
//! which the library's manifest says how to make (#737).
pub struct Held<I> {
    pub iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

pub fn held<H>(iters: H) -> Vec<Held<<H::Item as IntoIterator>::IntoIter>>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    iters.map(|i| Held { iter: i.into_iter() }).collect()
}

/// Of a tuple of its type parameter's item, an array and a reference.
pub fn tagged<I: Iterator>(items: I) -> Vec<(I::Item, [u8; 3], &'static u16)> {
    items.map(|item| (item, [7; 3], &0)).collect()
}

/// What its own crate drops, so its code takes the drops of what it's
/// given, and a consumer calls its JS, made of the manifest's template.
pub struct Loud(pub u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn warm() -> usize {
    held(vec![vec![Loud(1)]].into_iter()).len() + tagged([Loud(2)].into_iter()).len()
}
