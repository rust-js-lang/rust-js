// `is_sorted()` and `is_sorted_by(f)` of an iterator's items with a
// destructor, taken by value: each kept as `last` till the next replaces
// it and dropped then, where two are out of order the later dropped, then
// the last, then what's left of the iterator, as std's `is_sorted_by`
// drops them (#948). Of sorted, unsorted and no items, through `by_ref()`,
// whose rest stays, and as a comparison panics, which drops what's held
// as it unwinds. Of a type parameter's iterator too, a `repeat`'s `take`,
// whose value, apart from its clones, is dropped with the iterator, after
// the last.
use std::panic;
#[derive(Debug, PartialEq, PartialOrd)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
#[derive(Debug, PartialEq, PartialOrd)]
struct C(u32);
impl Clone for C {
    fn clone(&self) -> Self {
        C(self.0 + 100)
    }
}
impl Drop for C {
    fn drop(&mut self) {
        println!("drop c {}", self.0);
    }
}
fn ds(v: &[u32]) -> Vec<D> {
    v.iter().map(|&x| D(x)).collect()
}
fn sorted<I: Iterator<Item = C>>(it: I) -> bool {
    it.is_sorted()
}
fn sorted_by<I: Iterator<Item = C>>(it: I) -> bool {
    it.is_sorted_by(|a, b| a.0 <= b.0)
}
fn main() {
    println!("a {}", ds(&[1, 2, 3]).into_iter().is_sorted());
    println!("b {}", ds(&[1, 3, 2, 4]).into_iter().is_sorted());
    println!("c {}", ds(&[]).into_iter().is_sorted());
    println!("d {}", ds(&[3, 2, 1]).into_iter().is_sorted_by(|a, b| a.0 >= b.0));
    println!("e {}", ds(&[1, 5, 2, 6]).into_iter().is_sorted_by(|a, b| a.0 <= b.0));
    let mut it = ds(&[1, 2, 0, 7, 8]).into_iter();
    println!("f {}", it.by_ref().is_sorted());
    println!("f rest {:?}", it.next());
    let r = panic::catch_unwind(|| {
        ds(&[1, 2, 3, 4]).into_iter().is_sorted_by(|a, b| {
            if b.0 == 3 {
                panic!("cmp");
            }
            a.0 <= b.0
        })
    });
    println!("g {}", r.is_err());
    println!("h {}", sorted(std::iter::repeat(C(7)).take(3)));
    println!("i {}", sorted_by(std::iter::repeat(C(8)).take(2)));
    println!("end");
}
