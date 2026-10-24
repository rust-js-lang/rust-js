// A `&mut` to one of std's enums whose variants have fields but for some,
// `Bound`'s `Unbounded`: the enum's object, which `*r = ..` and a change of
// what a variant holds write in place.

use std::ops::Bound;

fn widen(bound: &mut Bound<u32>) {
    if let Bound::Included(n) | Bound::Excluded(n) = bound {
        *n += 1;
    }
}

fn clear(bound: &mut Bound<u32>) {
    *bound = Bound::Unbounded;
}

fn main() {
    let mut bound = Bound::Included(3);
    widen(&mut bound);
    println!("{bound:?}");
    clear(&mut bound);
    widen(&mut bound);
    println!("{bound:?}");
    let mut bounds = vec![Bound::Excluded(1), Bound::Unbounded, Bound::Included(9)];
    for bound in bounds.iter_mut() {
        widen(bound);
    }
    clear(&mut bounds[2]);
    println!("{bounds:?}");
}
