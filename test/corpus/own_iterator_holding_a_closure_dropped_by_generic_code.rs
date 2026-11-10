// The crate's own iterator, holding an adapter of a closure, given to
// generic code as a JS iterator, `$iterator(value, next)`, with the drop
// that drops the value it was made of, its `of` (ADR 0421): its type was
// told apart from itself by a region in the closure's type, so it was given
// the value's own drop, which read the JS iterator as the struct (#1239).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own<I>(I);

impl<I: Iterator> Iterator for Own<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

fn even(l: &Loud) -> bool {
    l.0 % 2 == 0
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn first<I: Iterator>(mut it: I) -> Option<I::Item> {
    it.next()
}

fn main() {
    let a = vec![Loud(10), Loud(11), Loud(12)];
    let b = vec![Loud(2), Loud(3), Loud(4)];
    let got: Vec<(u8, u8)> = collected(Own(a.into_iter().map(|l| l.0).zip(b.into_iter().take_while(even).map(|l| l.0))));
    println!("{got:?}");
    let c = vec![Loud(20), Loud(21)];
    println!("{:?}", first(Own(c.into_iter().map(|l| l.0 + 1))));
}
