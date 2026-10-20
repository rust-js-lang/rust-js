// What `extend`, `chain`, `zip` and a `Debug` builder's `entries` take is
// any `IntoIterator`: an `Option`'s items are none or its one, and a range's
// its numbers, as JS steps them (ADR 0528).
use std::fmt;

struct Listed(Option<i32>, std::ops::Range<i32>);

impl fmt::Debug for Listed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.0).entries(self.1.clone()).finish()
    }
}

fn main() {
    let some = Some(4);
    let none: Option<i32> = None;
    let mut v = vec![1];
    v.extend(some);
    v.extend(none);
    v.extend(3..5);
    let chained: Vec<i32> = (0..2).chain(5..7).chain(some).chain(none).collect();
    let zipped: Vec<(i32, i32)> = (0..3).zip(10..12).collect();
    let with_some: Vec<(i32, i32)> = vec![7, 8].into_iter().zip(some).collect();
    println!("{v:?} {chained:?} {zipped:?} {with_some:?}");
    println!("{:?} {:?}", Listed(Some(5), 1..3), Listed(None, 0..0));
}
