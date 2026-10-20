// A std adapter of a type parameter given as it is, `inner.map(f)`, is one
// of the JS iterator made of it, which knows its back where it's given its
// `DoubleEndedIterator` dictionary, held in a variable or not: `rfold` of it,
// where a function with a destructor makes rust-js hold the map first, steps
// it from its back (ADR 0488).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Tag<T>(T);

struct Ends<I> {
    inner: I,
}

impl<I: DoubleEndedIterator> Ends<I> {
    fn tagged_back<A, G>(self, init: A, f: G) -> A
    where
        G: FnMut(A, Tag<I::Item>) -> A,
    {
        self.inner.map(Tag).rfold(init, f)
    }
}

fn main() {
    let loud = Loud("fold");
    let tags = Ends { inner: 0..3 }.tagged_back(Vec::new(), move |mut v, t| {
        v.push((t, loud.0));
        v
    });
    println!("{tags:?}");
    let words = Ends { inner: vec!["x", "y"].into_iter() }.tagged_back(String::new(), |s, t| s + t.0);
    println!("{words}");
}
