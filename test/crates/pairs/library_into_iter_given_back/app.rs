//! Each read as the type it is: the ranges given back, the library's `Bag`'s
//! and the app's own `&Mine`'s, whose dictionary the library's `total`
//! steps.
use dep::{Bag, it, some, total};

struct Mine(u32);

impl IntoIterator for &Mine {
    type Item = u32;
    type IntoIter = std::ops::Range<u32>;
    fn into_iter(self) -> std::ops::Range<u32> {
        0..self.0
    }
}

pub fn main() {
    let r = it(3..6);
    println!("{:?} {}", r, r.start);
    let b = it(Bag(vec![1, 2]));
    println!("{:?} {}", b, b.end);
    let s = some(Bag(vec![1])).unwrap();
    println!("{:?}", s);
    println!("{}", total(&Mine(4)));
    let m = it(&Mine(2));
    println!("{:?}", m);
}
