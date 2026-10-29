// A destructor that panics leaves the parts after it still dropped, as
// Rust's drop glue goes on: a struct's fields, a tuple's, a variant's, an
// array's and a `Vec`'s items, a map's entries, and the fields of a type
// whose own `drop` panics (#416).

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Boom(u32);
impl Drop for Boom {
    fn drop(&mut self) {
        println!("boom {}", self.0);
        panic!("boom");
    }
}

struct Pair {
    first: Boom,
    second: Loud,
}

enum Shape {
    Three(Loud, Boom, Loud),
    Empty,
}

struct Panics {
    inner: Loud,
}
impl Drop for Panics {
    fn drop(&mut self) {
        println!("panics");
        panic!("own drop");
    }
}

enum Item {
    Quiet(Loud),
    Loud(Boom),
}

impl Item {
    fn id(&self) -> u32 {
        match self {
            Item::Quiet(l) => l.0,
            Item::Loud(b) => b.0,
        }
    }
}

fn caught(name: &str, f: impl FnOnce()) {
    let result = catch_unwind(AssertUnwindSafe(f));
    println!("{name} caught {}", result.is_err());
}

fn main() {
    caught("struct", || {
        let pair = Pair { first: Boom(1), second: Loud(2) };
        println!("pair {} {}", pair.first.0, pair.second.0);
    });
    caught("tuple", || {
        let _t = (Loud(3), Boom(4), Loud(5));
    });
    caught("variant", || {
        let s = Shape::Three(Loud(6), Boom(7), Loud(8));
        let e = Shape::Empty;
        for shape in [&s, &e] {
            if let Shape::Three(a, b, c) = shape {
                println!("three {} {} {}", a.0, b.0, c.0);
            }
        }
    });
    caught("array", || {
        let a = [Item::Quiet(Loud(9)), Item::Loud(Boom(10)), Item::Quiet(Loud(11))];
        println!("array {}", a.iter().map(Item::id).sum::<u32>());
    });
    caught("vec", || {
        let v = vec![Item::Quiet(Loud(12)), Item::Loud(Boom(13)), Item::Quiet(Loud(14))];
        println!("vec {}", v.len());
    });
    caught("map", || {
        let mut m = BTreeMap::new();
        m.insert(1, Item::Quiet(Loud(15)));
        m.insert(2, Item::Loud(Boom(16)));
        m.insert(3, Item::Quiet(Loud(17)));
    });
    caught("own drop", || {
        let p = Panics { inner: Loud(18) };
        println!("inner {}", p.inner.0);
    });
    println!("done");
}
