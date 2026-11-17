//@ library-differs
// A trait method whose result is an `impl DoubleEndedIterator`, a `map` of
// a slice's items its impl makes: stepped from its back by generic code,
// through the impl's dictionary, and where it's called on its type, as
// the type its impl's hides, std's `Map` (#2022). Its signature's own
// lifetime, which the result holds, never reaches a query bound.
pub trait Source {
    fn items(&self) -> impl DoubleEndedIterator<Item = u32> + '_;
    fn sized(&self) -> impl ExactSizeIterator<Item = u32> + DoubleEndedIterator + '_;
}

struct A(Vec<u32>);

impl Source for A {
    fn items(&self) -> impl DoubleEndedIterator<Item = u32> + '_ {
        self.0.iter().map(|x| x * 10)
    }

    fn sized(&self) -> impl ExactSizeIterator<Item = u32> + DoubleEndedIterator + '_ {
        self.0.iter().copied().rev()
    }
}

fn source() -> impl Source {
    A(vec![1, 2, 3])
}

fn last<S: Source>(s: &S) -> Option<u32> {
    s.items().next_back()
}

fn len_and_back<S: Source>(s: &S) -> (usize, Option<u32>) {
    let mut it = s.sized();
    (it.len(), it.next_back())
}

fn main() {
    let s = source();
    println!("{:?} {:?}", last(&s), len_and_back(&s));
    println!("{:?}", s.items().rev().collect::<Vec<_>>());
    let mut both = s.items();
    println!("{:?} {:?} {:?}", both.next(), both.next_back(), both.collect::<Vec<_>>());
    println!("{:?} {}", s.sized().rev().collect::<Vec<_>>(), s.sized().len());
}
