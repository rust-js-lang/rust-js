// A place reached through a reference an array's item holds, borrowed
// `&mut`, then the item given another reference before the borrow's used:
// `&mut cur[0].count` is of `a`, as Rust's borrow keeps the place it was
// made of, where a handle made of it read `cur[0]` again where it was read
// (#1075, the metamorphic net's `array` rewrite of `mut_ref_local`), as one
// through a field's reference did before #988 and #1034; and at an index
// a variable holds, each item given another; and a `Cell` set through
// such an item, the item given another while what it's set to is made,
// `node[0].v.set({ node[0] = &b; .. })`, as #1034's through a field.
use std::cell::Cell;

struct Node {
    count: i32,
}

struct Shared {
    v: Cell<u32>,
}

fn main() {
    let mut a = Node { count: 1 };
    let mut b = Node { count: 10 };
    let mut cur = [&mut a];
    let c = [&mut cur[0].count];
    cur[0] = &mut b;
    *c[0] += 1;
    cur[0].count += 5;
    println!("{} {}", a.count, b.count);

    let (mut x, mut y) = (Node { count: 2 }, Node { count: 20 });
    let (mut s, mut t) = (Node { count: 7 }, Node { count: 70 });
    let mut refs = [&mut x, &mut y];
    let i = 1;
    let held = (&mut refs[i].count,);
    refs[0] = &mut s;
    refs[1] = &mut t;
    *held.0 += 3;
    println!("{} {} {} {}", x.count, y.count, s.count, t.count);

    let (e, f) = (Shared { v: Cell::new(10) }, Shared { v: Cell::new(10) });
    let mut node = [&e];
    node[0].v.set({
        node[0] = &f;
        node[0].v.get() + 1
    });
    println!("{} {}", e.v.get(), f.v.get());
}
