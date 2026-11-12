//@ library-differs
// An iterator's own `fold`, holding its accumulator in a `Some`, which
// std's provided methods call too, `last()` through rust-js's `$folded`,
// with an accumulator of their own the crate never names: a std trait's
// method's callers aren't the crate's alone, so its `Some` of `B` stays a
// box whatever the crate's own calls give (#1233).
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        let mut acc = Some(init);
        while let Some(x) = self.next() {
            acc = Some(f(acc.take().unwrap(), x));
        }
        acc.unwrap()
    }
}

fn main() {
    println!("{}", Counter(4).fold(0u32, |a, x| a + x));
    println!("{:?} {:?}", Counter(3).last(), Counter(0).last());
}
