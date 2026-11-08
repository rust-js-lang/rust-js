// A place reached through a reference a field holds, borrowed, then the
// reference set again before the borrow's used: `node.0.v.set({ node.0 =
// b; .. })` sets `a`'s, as Rust's borrow keeps the place it was made of,
// where the field was read again after the write (#1034). The reference
// set again through a box the field holds, `*held.0 = d`, the box set
// again, `held.0 = Box::new(c)`, the holder set again whole, `node =
// Plain(b)`, a call's result written there, or a closure lent it, `|| node.0
// = b`, which a variable that's the reference, lent so, was missed by too;
// and a `&mut` through each (#988).
use std::cell::Cell;

struct Node {
    v: Cell<u32>,
}

struct Plain<'a>(&'a Node);

struct Boxed<'a>(Box<&'a Node>);

struct Pair<'a> {
    left: &'a Node,
    tag: u32,
}

struct Counter {
    count: u32,
}

struct Holder<'a>(Box<&'a mut Counter>);

struct Walk<'a> {
    at: &'a mut Counter,
}

fn other(n: &Node) -> &Node {
    n
}

fn bump(c: &mut u32, by: u32) {
    *c += by;
}

fn main() {
    let a = Node { v: Cell::new(10) };
    let b = Node { v: Cell::new(10) };
    let mut node = Plain(&a);
    node.0.v.set({
        node.0 = &b;
        node.0.v.get() + 1
    });
    println!("{} {}", a.v.get(), b.v.get());

    let c = Node { v: Cell::new(20) };
    let d = Node { v: Cell::new(20) };
    let mut held = Boxed(Box::new(&c));
    held.0.v.set({
        *held.0 = &d;
        held.0.v.get() + 1
    });
    println!("{} {}", c.v.get(), d.v.get());
    let mut reboxed = Boxed(Box::new(&d));
    reboxed.0.v.set({
        reboxed.0 = Box::new(&c);
        reboxed.0.v.get() + 2
    });
    println!("{} {}", c.v.get(), d.v.get());

    let mut whole = Plain(&a);
    whole.0.v.set({
        whole = Plain(&b);
        whole.0.v.get() + 1
    });
    println!("{} {}", a.v.get(), b.v.get());

    let mut pair = Pair { left: &a, tag: 1 };
    pair.left.v.set({
        pair.left = other(&b);
        pair.tag += 1;
        pair.left.v.get() + pair.tag
    });
    println!("{} {} {}", a.v.get(), b.v.get(), pair.tag);

    // Set again by a closure that's lent the field.
    let mut lent = Plain(&a);
    let kept = &lent.0.v;
    let mut swap = || lent.0 = &b;
    swap();
    kept.set(kept.get() + 10);
    println!("{} {} {}", a.v.get(), b.v.get(), lent.0.v.get());
    let mut lent = Plain(&a);
    lent.0.v.set({
        let mut swap = || lent.0 = &b;
        swap();
        20
    });
    println!("{} {}", a.v.get(), b.v.get());
    // And a reference a variable is, lent to be set again.
    let mut local = &a;
    local.v.set({
        let mut swap = || local = &b;
        swap();
        30
    });
    let mut local = &a;
    local.v.set({
        let r = &mut local;
        *r = &b;
        40
    });
    println!("{} {}", a.v.get(), b.v.get());

    // Read and set before the field's set again: nothing held.
    let mut read = Plain(&a);
    read.0.v.set(read.0.v.get() + 3);
    read.0 = &b;
    read.0.v.set(read.0.v.get() + 4);
    println!("{} {}", a.v.get(), b.v.get());

    let nodes = [
        Node { v: Cell::new(1) },
        Node { v: Cell::new(2) },
        Node { v: Cell::new(3) },
    ];
    let mut cur = Plain(&nodes[0]);
    for next in nodes.iter().skip(1) {
        cur.0.v.set({
            let was = cur.0.v.get();
            cur.0 = next;
            was + cur.0.v.get()
        });
    }
    println!("{:?}", nodes.iter().map(|n| n.v.get()).collect::<Vec<_>>());

    let mut e = Counter { count: 1 };
    let mut f = Counter { count: 2 };
    let mut h = Holder(Box::new(&mut e));
    let r = &mut h.0.count;
    *h.0 = &mut f;
    *r += 10;
    h.0.count += 100;
    println!("{} {}", e.count, f.count);

    let mut g = Counter { count: 1 };
    let mut k = Counter { count: 2 };
    let mut w = Walk { at: &mut g };
    let s = &mut w.at.count;
    w = Walk { at: &mut k };
    bump(s, 5);
    w.at.count += 7;
    println!("{} {}", g.count, k.count);
}
