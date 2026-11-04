// `last()` of a `Vec`'s `into_iter()` is std's `IntoIter::last`,
// `next_back()`: it takes the last item, then drops what it has left, front
// to back, as it's dropped. Where one of those drops panics, the rest are
// still dropped, and the item it took is leaked, as rustc doesn't drop
// what a function gives back as a later drop unwinds (#648). In place,
// through a `skip()` and a `chain()`, each of which asks its source's own
// `last`, and of an array's and a `VecDeque`'s, whose `last` is theirs too.
// So too through a `fuse()` and a `peekable()`, whose `last` asks its
// source's, and in generic code, given one as its type parameter, which
// can't tell it from a `Drain` but by what its caller says of it.
// A `BinaryHeap`'s has none, so its `last` folds it, and its `nth_back`
// drops each it passes from the back, as std's defaults do.
use std::collections::{BinaryHeap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn last_of<I: Iterator<Item = D>>(it: I) -> Option<u32> {
    it.last().map(|d| d.0 + 0)
}

fn run(name: &str, f: impl FnOnce() -> Option<u32>) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("in place", || ds().into_iter().last().map(|d| d.0 + 0));
    run("skipped", || ds().into_iter().skip(1).last().map(|d| d.0 + 0));
    run("chained", || {
        ds().into_iter()
            .chain(vec![D(6)])
            .last()
            .map(|d| d.0 + 0)
    });
    run("chained, b's last panics", || {
        vec![D(1)]
            .into_iter()
            .chain(vec![D(2), D(3), D(4)])
            .last()
            .map(|d| d.0 + 0)
    });
    run("fused", || ds().into_iter().fuse().last().map(|d| d.0 + 0));
    run("peeked", || {
        let mut it = ds().into_iter().peekable();
        println!("peeked {:?}", it.peek().map(|d| d.0 + 0));
        it.last().map(|d| d.0 + 0)
    });
    run("generic", || last_of(ds().into_iter()));
    run("generic, skipped and fused", || last_of(ds().into_iter().skip(1).fuse()));
    run("generic, a drain's", || {
        let mut v = ds();
        let r = last_of(v.drain(..));
        println!("left {}", v.len());
        r
    });
    run("flattened", || {
        ds().into_iter()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .last()
            .map(|d| d.0 + 0)
    });
    run("flat-mapped, an inner's last panics", || {
        vec![vec![D(1), D(2)], vec![D(3), D(4)], vec![D(5)]]
            .into_iter()
            .flat_map(|v| v)
            .last()
            .map(|d| d.0 + 0)
    });
    run("flattened, mid-way", || {
        let mut it = ds().into_iter().map(|x| vec![D(x.0 + 10), x]).flatten();
        println!("first {:?}", it.next().map(|d| d.0 + 0));
        it.last().map(|d| d.0 + 0)
    });
    run("flattened, the one it's in panics", || {
        let mut it = vec![D(3), D(4)]
            .into_iter()
            .map(|x| {
                let n = x.0;
                vec![D(n + 10), x, D(n + 20)]
            })
            .flatten();
        println!("first {:?}", it.next().map(|d| d.0 + 0));
        it.last().map(|d| d.0 + 0)
    });
    run("flat-mapped, the one it's in panics", || {
        let mut it = vec![vec![D(1), D(3), D(4)], vec![D(5)]]
            .into_iter()
            .flat_map(|v| v);
        println!("first {:?}", it.next().map(|d| d.0 + 0));
        it.last().map(|d| d.0 + 0)
    });
    run("flat-mapped, its function panics", || {
        vec![vec![D(1)], vec![D(2)], vec![D(4)]]
            .into_iter()
            .flat_map(|v| {
                if v[0].0 == 2 {
                    panic!("made");
                }
                v
            })
            .last()
            .map(|d| d.0 + 0)
    });
    run("none panics", || {
        vec![D(1), D(2), D(4)].into_iter().last().map(|d| d.0 + 0)
    });
    run("array", || {
        [D(1), D(2), D(3), D(4), D(5)]
            .into_iter()
            .last()
            .map(|d| d.0 + 0)
    });
    run("deque", || {
        VecDeque::from(ds()).into_iter().last().map(|d| d.0 + 0)
    });
    run("heap", || {
        BinaryHeap::from(ds()).into_iter().last().map(|d| d.0 + 0)
    });
    run("heap nth_back", || {
        BinaryHeap::from(vec![D(10), D(20), D(30), D(40), D(50)])
            .into_iter()
            .nth_back(2)
            .map(|d| d.0 + 0)
    });
}
