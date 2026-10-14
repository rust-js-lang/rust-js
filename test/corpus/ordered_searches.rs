// What JS's `<` orders as `Ord` does is one answer, `is_primitive_ord`'s:
// a slice of references to numbers and one of `Ordering`s are searched as
// numbers are, and an `Ordering`'s and a `&mut` number's `cmp` as theirs
// (the second architecture review: `binary_search` asked its own way, and
// refused both).

use std::cmp::Ordering;

fn main() {
    let refs = [&1, &3, &5];
    println!("{:?} {:?}", refs.binary_search(&&3), refs.binary_search(&&4));
    let orders = [Ordering::Less, Ordering::Equal, Ordering::Greater];
    println!("{:?}", orders.binary_search(&Ordering::Greater));
    println!("{:?}", Ordering::Less.cmp(&Ordering::Greater));
    let mut a = 5;
    let mut b = 3;
    let ra = &mut a;
    let rb = &mut b;
    *ra += 1;
    *rb += 1;
    println!("{:?}", ra.cmp(&rb));
}
