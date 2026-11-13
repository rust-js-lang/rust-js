// The hint of a held iterator whose adapters hold others made in place, a
// `filter` of a `skip(1)`, a `take(2)` of a `cycle()` of a `map`, a
// `filter` of a `take(2)` of a map's values: each made knowing its hint,
// `$skipping`, as JS's own `drop(1)` keeps none, so `size_hint()` is
// std's, as it steps (#1427).

use std::collections::HashMap;

fn main() {
    let mut it = vec![1u8, 2, 3].into_iter().skip(1).filter(|x| *x > 1);
    println!("{:?}", it.size_hint());
    it.next();
    println!("{:?}", it.size_hint());
    let mut taken = vec![1u8, 2, 3].into_iter().take(2).filter(|x| *x > 0);
    println!("{:?}", taken.size_hint());
    taken.next();
    println!("{:?}", taken.size_hint());
    let mut c = [1u8, 2].iter().map(|x| x * 2).cycle().take(2);
    println!("{:?}", c.size_hint());
    c.next();
    println!("{:?} {:?}", c.size_hint(), c.next());
    // Each value kept, so what's stepped is the same whatever the map's
    // order, which Rust leaves unspecified.
    let n: HashMap<u32, String> = [(1, "a".to_string()), (2, "b".to_string()), (3, "c".to_string())].into();
    let mut values = n.into_values().take(2).filter(|s| !s.is_empty());
    println!("{:?}", values.size_hint());
    values.next();
    println!("{:?}", values.size_hint());
    // A `rev()` under them, which no hint asked makes `$skipping`, which
    // would know no back: its own form, `$rev`, keeps its source's hint.
    let r = [1u8, 2, 3];
    let mut back = r.iter().rev().skip(1).cycle().take(2);
    println!("{:?}", back.size_hint());
    back.next();
    println!("{:?}", back.size_hint());
    let e: [u8; 0] = [];
    let mut none = e.iter().rev().skip(1).cycle().take(2);
    println!("{:?} {:?}", none.size_hint(), none.next());
}
