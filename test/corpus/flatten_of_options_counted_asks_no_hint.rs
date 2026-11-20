//@ library-differs
// `count()` of a `flatten()` of `Option`s, or of a `flat_map` whose function
// returns one, is std's `FlattenCompat::count`, its source's `fold`, which
// never asks its source's `size_hint` (#2197). Where the crate asks the
// hint of its iterator's type elsewhere, a `zip`'s `len()` here, each one
// is made knowing its hint, so the flatten is made in place, a filter of
// the `Some`s; that filter's own `count` is std's `Filter::count`, which
// asks its source's hint first, where `Flatten`'s doesn't. A
// `filter_map`'s, std's default of its `fold`, asks none either.

/// Counts up to `.1` from `.0`, with its own `size_hint` and `fold`.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0 - 1)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

impl ExactSizeIterator for Own {}

fn main() {
    let zipped = Own(0, 3).zip(10..20u32);
    println!("zip len {}", zipped.len());

    let n = Own(0, 5).map(Some).flatten().count();
    println!("flatten count {n}");
    let n = Own(0, 5).flat_map(|x| (x % 2 == 0).then_some(x)).count();
    println!("flat_map count {n}");
    let n = Own(0, 5).filter_map(|x| (x % 2 == 1).then_some(x)).count();
    println!("filter_map count {n}");
    let n = Own(0, 5).filter(|x| x % 2 == 1).count();
    println!("filter count {n}");
}
