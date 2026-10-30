//@ library-differs
// An impl whose associated type is an iterator, `IntoIterator`'s `IntoIter`
// or a trait's own `type Out: Iterator`, steps what that type's `next`
// steps, its type parameter given as it is: its dictionary gives that
// type's, so it's given the parameter's (ADR 0527).
#[derive(Clone)]
pub struct Wrap<I> {
    inner: I,
}

impl<I: Iterator> Iterator for Wrap<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.inner.next()
    }
}

impl<I: Iterator + Clone> IntoIterator for &Wrap<I> {
    type Item = I::Item;
    type IntoIter = Wrap<I>;
    fn into_iter(self) -> Wrap<I> {
        self.clone()
    }
}

pub trait Fused<I> {
    type Out: Iterator + From<I>;
}

pub struct Marker;

impl<I: Iterator + Clone> Fused<I> for Marker {
    type Out = Wrap<std::iter::Fuse<I>>;
}

impl<I: Iterator + Clone> From<I> for Wrap<std::iter::Fuse<I>> {
    fn from(inner: I) -> Self {
        Wrap { inner: inner.fuse() }
    }
}

pub fn make<I, M: Fused<I>>(inner: I) -> M::Out {
    M::Out::from(inner)
}

pub fn wrap<I>(inner: I) -> Wrap<I> {
    Wrap { inner }
}

fn main() {
    let w = wrap(1..4);
    let mut total = 0;
    for x in &w {
        total += x;
    }
    println!("{total} {}", w.count());
    let fused: i32 = make::<_, Marker>(vec![5, 6].into_iter()).sum();
    println!("{fused}");
}
