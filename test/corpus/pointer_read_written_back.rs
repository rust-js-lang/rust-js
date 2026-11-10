// What a raw pointer points at, read and written back unchanged: the same
// map, a `Map` given itself to become in place, which clearing it first
// would empty (#1109). Of a local's place and of a `Vec`'s item.
use std::collections::{HashMap, HashSet};

fn main() {
    let mut m = HashMap::from([(1, 10), (2, 20)]);
    let p: *mut HashMap<u32, u32> = &mut m;
    // SAFETY: `p` points at `m`, read and written back before `m` is used.
    unsafe {
        let held = p.read();
        p.write(held);
    }
    println!("{}", m.len());
    let mut sets = vec![HashSet::from([3, 4, 5])];
    let q: *mut HashSet<u32> = &mut sets[0];
    // SAFETY: as above, of the `Vec`'s first item.
    unsafe {
        let held = std::ptr::read(q);
        std::ptr::write(q, held);
    }
    println!("{}", sets[0].len());
}
