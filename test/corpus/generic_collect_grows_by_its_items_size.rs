//@ library-differs
// Generic code's `collect()` into a `Vec` of a type parameter's items asks
// its iterator's hint again each time the `Vec` is full, as std's
// `extend_desugared` does: its capacity starts at `MIN_NON_ZERO_CAP` by the
// items' size (8 of 1 byte, 4 of up to 1024, else 1) and doubles, and of
// items of no size it's never full. That size is its caller's, `TSize`,
// which a generic caller passes on as it's given it (#796).
use std::collections::VecDeque;

/// A crate's iterator of `end` items made by `make`, whose `size_hint`
/// prints and says it knows nothing.
struct Own<F> {
    at: u32,
    end: u32,
    make: F,
}

impl<T, F: FnMut(u32) -> T> Iterator for Own<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.at += 1;
        (self.at <= self.end).then(|| (self.make)(self.at))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.at);
        (0, None)
    }
}

fn own<T>(end: u32, make: impl FnMut(u32) -> T) -> Own<impl FnMut(u32) -> T> {
    Own { at: 0, end, make }
}

/// Of `T`'s items, the caller's.
fn all<T, I: Iterator<Item = T>>(it: I) -> Vec<T> {
    it.collect()
}

/// Of `I::Item`s, the caller's.
fn items<I: Iterator>(it: I) -> Vec<I::Item> {
    it.collect()
}

/// Of a filter's, which passes on its source's hint's upper bound.
fn kept<T, I: Iterator<Item = T>>(it: I, keep: impl FnMut(&T) -> bool) -> Vec<T> {
    it.filter(keep).collect()
}

/// A generic caller's, which gives its own caller's size on.
fn relayed<U, J: Iterator<Item = U>>(it: J) -> usize {
    all(it).len()
}

/// And one more, of an associated type's.
fn relayed_twice<J: Iterator>(it: J) -> usize {
    relayed(it.map(|x| x)) + items(std::iter::empty::<J::Item>()).len()
}

/// Into what std collects as a `Vec` first: a boxed slice, a `VecDeque`.
fn boxed<T, I: Iterator<Item = T>>(it: I) -> Box<[T]> {
    it.collect()
}

fn queued<T, I: Iterator<Item = T>>(it: I) -> VecDeque<T> {
    it.collect()
}

/// Of items built of a type parameter, whose layout its caller gives.
fn tagged<T, I: Iterator<Item = T>>(it: I) -> Vec<(T, u8)> {
    it.map(|x| (x, 0)).collect()
}

fn wrapped<I: Iterator>(it: I) -> Vec<Option<I::Item>> {
    it.map(Some).collect()
}

/// And a generic caller's, which passes its own caller's on.
fn tagged_relayed<U, J: Iterator<Item = U>>(it: J) -> usize {
    tagged(it).len()
}

/// Into a B-tree, which std collects as a `Vec` first.
fn sorted_set<T: Ord, I: Iterator<Item = T>>(it: I) -> std::collections::BTreeSet<T> {
    it.collect()
}

fn sorted_map<K: Ord, V, I: Iterator<Item = (K, V)>>(it: I) -> std::collections::BTreeMap<K, V> {
    it.collect()
}

/// Of what has no end, whose room std asks first and can't give: a
/// `capacity overflow`, of no size asked, as its hint is std's own.
fn endless<T: Default>() -> usize {
    (0u32..).map(|_| T::default()).collect::<Vec<T>>().len()
}

/// A trait's default, copied into its blanket impl.
trait Gathered: Iterator + Sized {
    fn gathered(self) -> Vec<Self::Item> {
        self.collect()
    }
}

impl<I: Iterator> Gathered for I {}

fn main() {
    println!("bytes {}", all(own(20, |n| n as u8)).len());
    println!("words {}", all(own(20, |n| n)).len());
    println!("longs {}", all(own(20, u64::from)).len());
    println!("of no size {}", all(own(20, |_| ())).len());
    println!("big {}", all(own(5, |n| [n as u8; 2000])).len());
    println!("associated, shorts {}", items(own(20, |n| n as u16)).len());
    println!("filtered {}", kept(own(20, |n| n), |n| n % 2 == 0).len());
    println!("relayed, bytes {}", relayed(own(20, |n| n as u8)));
    println!("relayed, longs {}", relayed(own(20, u64::from)));
    println!("relayed twice, pairs {}", relayed_twice(own(12, |n| (n, n))));
    println!("relayed twice, of no size {}", relayed_twice(own(12, |_| ())));
    println!("of a range {}", all(1..20u8).len());
    println!("boxed {}", boxed(own(10, |n| n)).len());
    println!("queued {}", queued(own(10, |n| n as u8)).len());
    println!("tagged bytes {}", tagged(own(20, |n| n as u8)).len());
    println!("tagged longs {}", tagged(own(20, u64::from)).len());
    println!("wrapped words {}", wrapped(own(20, |n| n)).len());
    println!("wrapped of no size {}", wrapped(own(20, |_| ())).len());
    println!("tagged, relayed {}", tagged_relayed(own(20, |n| n as u16)));
    println!("sorted set {}", sorted_set(own(10, |n| n as u8)).len());
    println!("sorted map {}", sorted_map(own(10, |n| (n, u64::from(n)))).len());
    println!("gathered {}", own(10, |n| n).gathered().len());
    let overflowed = std::panic::catch_unwind(endless::<u32>);
    println!("endless {:?}", overflowed.ok());
}
