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
    // A `Vec`'s items it drops itself, `truncate`'s and `clear`'s, a
    // receiver's queue, and what a failed `collect` of options took, each
    // dropped though one before panics, as std's slice drop goes on.
    caught("truncate", || {
        let mut v = vec![Item::Quiet(Loud(19)), Item::Loud(Boom(20)), Item::Quiet(Loud(21))];
        v.truncate(0);
    });
    caught("clear", || {
        let mut v = vec![Item::Loud(Boom(22)), Item::Quiet(Loud(23))];
        v.clear();
    });
    caught("queue", || {
        let (send, receive) = std::sync::mpsc::channel();
        send.send(Item::Loud(Boom(24))).unwrap();
        send.send(Item::Quiet(Loud(25))).unwrap();
        drop(receive);
    });
    caught("collect", || {
        let items = vec![Some(Item::Loud(Boom(26))), Some(Item::Quiet(Loud(27))), None];
        let all: Option<Vec<Item>> = items.into_iter().collect();
        println!("collected {}", all.is_some());
    });
    caught("nth_back", || {
        let mut it = vec![Item::Quiet(Loud(28)), Item::Quiet(Loud(29)), Item::Loud(Boom(30)), Item::Quiet(Loud(31))]
            .into_iter();
        println!("nth_back {}", it.nth_back(2).map_or(0, |item| item.id()));
    });
    // A `retain` that drops one that panics, or whose test panics: what's
    // left is what it kept then what it hadn't tested, as std's guard puts
    // them back, each dropped once.
    caught("retain", || {
        let mut v = vec![Item::Quiet(Loud(32)), Item::Loud(Boom(33)), Item::Quiet(Loud(34))];
        v.retain(|item| matches!(item, Item::Quiet(_)));
    });
    caught("retain_mut", || {
        let mut v = vec![Item::Loud(Boom(35)), Item::Quiet(Loud(36))];
        v.retain_mut(|item| matches!(item, Item::Quiet(_)));
    });
    caught("retain test", || {
        let mut v = vec![Loud(37), Loud(38), Loud(39)];
        v.retain(|l| if l.0 == 38 { panic!("test") } else { true });
    });
    caught("dedup", || {
        let mut v = vec![Item::Quiet(Loud(43)), Item::Loud(Boom(44)), Item::Quiet(Loud(45))];
        v.dedup_by(|a, b| matches!((a, b), (Item::Loud(_), Item::Quiet(_))));
    });
    caught("heap retain", || {
        let mut heap = std::collections::BinaryHeap::new();
        for n in [40u32, 42, 41] {
            heap.push(n);
        }
        let mut seen = Vec::new();
        heap.retain(|&n| {
            seen.push(n);
            if n == 42 { panic!("heap") } else { true }
        });
        println!("heap {:?}", seen);
    });
    caught("own drop", || {
        let p = Panics { inner: Loud(18) };
        println!("inner {}", p.inner.0);
    });
    println!("done");
}
