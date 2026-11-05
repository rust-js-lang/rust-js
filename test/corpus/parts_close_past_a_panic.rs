// An adapter of two or more parts dropped part-way, the first part's drop
// panicking: each part after it is dropped as it unwinds, in std's field
// order, as drop glue goes on past a panicking field (#743): a chain's
// `a`, then `b`; a zip's `a`, then `b`; a flatten's source, then its front
// and back inners; a chain stepped from its back elsewhere in the crate,
// made knowing its back; a comparison's two iterators; and a map's source,
// then its function (#762).
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

#[derive(PartialEq)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn boom(what: &str, n: u32, run: impl FnOnce() + std::panic::UnwindSafe) {
    BOOM.set(n);
    let r = catch_unwind(run);
    println!("{what} ok {}", r.is_ok());
}

// A chain stepped from its back here, so each of its type is made knowing
// its back.
fn last_two(v: Vec<D>, w: Vec<D>) -> Option<u32> {
    v.into_iter().chain(w).nth_back(1).map(|d| d.0)
}

type Chained = std::iter::Chain<std::vec::IntoIter<D>, std::vec::IntoIter<D>>;

fn chained(v: Vec<D>, w: Vec<D>) -> Chained {
    v.into_iter().chain(w)
}

fn main() {
    boom("chain", 3, || {
        let mut it = ds().into_iter().chain(vec![D(6), D(7)]);
        it.next();
        drop(it);
    });
    boom("zip", 3, || {
        let mut it = ds().into_iter().zip(vec![D(16), D(17), D(18)]);
        it.next();
        drop(it);
    });
    boom("flatten", 3, || {
        let mut it = vec![ds(), vec![D(6), D(7)], vec![D(8), D(9)]].into_iter().flatten();
        it.next();
        it.next_back();
        drop(it);
    });
    println!("{:?}", last_two(vec![D(21)], vec![D(22)]));
    boom("chain knowing its back", 3, || {
        let mut it = chained(ds(), vec![D(6), D(7)]);
        it.next();
        drop(it);
    });
    boom("chain from its back", 2, || {
        let mut it = chained(ds(), vec![D(6), D(7), D(8)]);
        it.next_back();
        drop(it);
    });
    // A comparison's two iterators, each dropped though the other's drop
    // panics.
    for n in [7, 3] {
        boom("cmp", n, || {
            let other = vec![D(1), D(9), D(7)].into_iter().map(|d| d.0);
            println!("{:?}", ds().into_iter().map(|d| d.0).cmp(other));
        });
        boom("eq", n, || {
            let other = vec![D(1), D(9), D(7)].into_iter().filter(|d| d.0 > 0);
            println!("{:?}", ds().into_iter().filter(|d| d.0 > 0).eq(other));
        });
    }
    // A map's function that holds what drops, after its source, though the
    // source's drop panics (#762).
    boom("map holding", 3, || {
        let h = D(80);
        let mut it = ds().into_iter().map(move |x| {
            let _h = &h;
            D(x.0 + 100)
        });
        let first = it.next();
        drop(it);
        drop(first);
    });
}
