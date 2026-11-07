// std's own `clone_from` of a `VecDeque`, a `LinkedList`, a `HashMap`, a
// `HashSet`, a `BinaryHeap`, an array, an `Option` and a `Box`, each in
// std's order of clones and drops, not `*dest = source.clone()`'s, every
// clone then the old value dropped (#922): a `VecDeque`'s clears, then
// clones; a `LinkedList`'s writes over what it keeps, pops the rest, then
// clones; a hashed map's drops its entries, then clones; an array's
// writes each item over; an `Option`'s and a `Box`'s are their value's.
// As a clone panics, what std's guard keeps is kept.
use std::cell::Cell;
use std::collections::{BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

/// Prints as it's cloned and dropped, and panics as it's cloned where its
/// number is `BOOM`'s, once.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        if BOOM.get() == self.0 {
            BOOM.set(0);
            panic!("clone {}", self.0);
        }
        D(self.0)
    }
}

/// No destructor, and a clone that panics where its number is `BOOM`'s:
/// what a `clone_from` wrote before it panicked is seen after.
#[derive(Debug)]
struct P(u32);

impl Clone for P {
    fn clone(&self) -> P {
        if BOOM.get() == self.0 {
            BOOM.set(0);
            panic!("clone {}", self.0);
        }
        P(self.0)
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    for boom in [0, 1, 3] {
        run("VecDeque", boom, || {
            let v: VecDeque<D> = vec![D(3), D(1)].into();
            let mut w: VecDeque<D> = vec![D(7), D(8)].into();
            w.clone_from(&v);
            println!("{}", w.len());
        });
        run("Option<Vec>", boom, || {
            let v = Some(vec![D(3), D(1)]);
            let mut w = Some(vec![D(7)]);
            w.clone_from(&v);
            println!("{:?}", w);
            let mut none = None;
            none.clone_from(&v);
            println!("{:?}", none);
            let mut some = Some(vec![D(5)]);
            some.clone_from(&None);
            println!("{:?}", some);
        });
        run("Box<Vec>", boom, || {
            let v = Box::new(vec![D(3), D(1)]);
            let mut w = Box::new(vec![D(7)]);
            w.clone_from(&v);
            println!("{}", w.len());
        });
        run("Box<VecDeque>", boom, || {
            let v: Box<VecDeque<D>> = Box::new(vec![D(3), D(1)].into());
            let mut w: Box<VecDeque<D>> = Box::new(vec![D(7)].into());
            w.clone_from(&v);
            println!("{}", w.len());
        });
        run("array", boom, || {
            let v = [D(3), D(1)];
            let mut w = [D(7), D(8)];
            w.clone_from(&v);
            println!("{:?}", w);
        });
        run("LinkedList", boom, || {
            let v = LinkedList::from([D(3), D(1)]);
            let mut w = LinkedList::from([D(7), D(8), D(9)]);
            w.clone_from(&v);
            let mut x = LinkedList::from([D(6)]);
            x.clone_from(&v);
            println!("{} {}", w.len(), x.len());
        });
        run("array of Option<Vec>", boom, || {
            let v = [Some(vec![D(3)]), Some(vec![D(1)]), None];
            let mut w = [Some(vec![D(7), D(8)]), None, Some(vec![D(5)])];
            w.clone_from(&v);
            println!("{:?}", w);
        });
        run("BinaryHeap", boom, || {
            let v: BinaryHeap<D> = vec![D(3)].into();
            let mut w: BinaryHeap<D> = vec![D(7), D(8)].into();
            w.clone_from(&v);
            println!("{}", w.len());
        });
        // Of entries whose order a hashed map picks: each clones and
        // drops alike.
        run("HashMap", boom, || {
            let v = HashMap::from([(1, D(3)), (2, D(3))]);
            let mut w = HashMap::from([(9, D(7))]);
            w.clone_from(&v);
            println!("{}", w.len());
        });
        let mut w = vec![P(7), P(8)];
        run("Vec of no drop", boom, || w.clone_from(&vec![P(3), P(1), P(4)]));
        println!("{w:?}");
        let mut w: VecDeque<P> = vec![P(7)].into();
        run("VecDeque of no drop", boom, || w.clone_from(&vec![P(3), P(1)].into()));
        println!("{w:?}");
        let mut w = [P(7), P(8)];
        run("array of no drop", boom, || w.clone_from(&[P(3), P(1)]));
        println!("{w:?}");
        let mut w = HashMap::from([(9, P(7))]);
        run("HashMap of no drop", boom, || w.clone_from(&HashMap::from([(1, P(3))])));
        println!("{w:?}");
        run("HashSet", boom, || {
            let v = HashSet::from([D(3)]);
            let mut w = HashSet::from([D(7)]);
            w.clone_from(&v);
            println!("{}", w.len());
        });
    }
}
