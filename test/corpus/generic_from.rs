// `W::from(u)` of a `W: From<U>`, and of an associated type a trait bounds
// by `From`, `T::Made::from(v)`: its dictionary's `from`, its caller's, the
// crate's impl's or std's conversion, as `x.into()`'s is (ADR 0477).
struct Wrap(Vec<u8>);

impl From<Vec<u8>> for Wrap {
    fn from(v: Vec<u8>) -> Self {
        Wrap(v)
    }
}

trait Maker<I> {
    type Made: From<I>;
}

struct Unit;

impl Maker<Vec<u8>> for Unit {
    type Made = Wrap;
}

fn make<T: Maker<Vec<u8>>>(v: Vec<u8>) -> T::Made {
    T::Made::from(v)
}

fn build<U, W: From<U>>(u: U) -> W {
    W::from(u)
}

fn queued<T>(v: Vec<T>) -> std::collections::VecDeque<T> {
    std::collections::VecDeque::from(v)
}

fn conv<T: From<u8>>(x: u8) -> T {
    T::from(x)
}

fn widened<T: Into<u64>>(x: T) -> u64 {
    x.into()
}

fn main() {
    let w: Wrap = make::<Unit>(vec![1, 2]);
    println!("{:?}", w.0);
    let w2: Wrap = build(vec![3]);
    println!("{:?}", w2.0);
    let n: u32 = conv(7);
    let s: String = build("hi");
    println!("{n} {s}");
    println!("{:?}", queued(vec!['a', 'b']));
    println!("{} {} {}", widened(true), widened(7u32), widened('a'));
}
