// A `map` or a `filter` the crate clones, made again by a function of its
// source and its closure (ADR 0525), whose closure holds a value with a
// destructor: each one made, a clone too, drops its own closure as std's
// `Map` and `Filter` drop theirs, a clone's its closure's clone, never the
// original's (#2160). The original dropped alone; a clone dropped before
// it; a `cycle()`, which keeps a clone and replaces what it steps each
// round, a clone first, then the one it replaces, restarted twice, held by
// a variable and as a temporary taken from; and a drop that panics as the
// cycle restarts, the fresh clone and what it started with dropped as the
// panic unwinds.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static PANIC_DROP_OF: Cell<u32> = const { Cell::new(u32::MAX) };
}

struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Self {
        let n = CLONES.with(|c| c.get() + 1);
        CLONES.with(|c| c.set(n));
        println!("clone {} as {}", self.0, self.0 * 10 + n);
        Loud(self.0 * 10 + n)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == PANIC_DROP_OF.with(Cell::get) {
            panic!("drop {}", self.0);
        }
    }
}

fn main() {
    let a = Loud(1);
    let mut alone = vec![10, 20].into_iter().map(move |x| {
        let a = &a;
        x + a.0
    });
    println!("{:?}", alone.next());
    let copy = alone.clone();
    println!("{:?}", alone.next());
    drop(alone);
    println!("original gone");
    drop(copy);
    println!("clone gone");

    let b = Loud(2);
    let kept = vec![1, 2, 3].into_iter().filter(move |x| {
        let b = &b;
        x % 2 == b.0 % 2
    });
    let mut copy = kept.clone();
    println!("{:?}", copy.next());
    drop(copy);
    println!("clone gone");
    println!("{:?}", kept.collect::<Vec<_>>());

    let c = Loud(3);
    let mut cycled = vec![10, 20]
        .into_iter()
        .map(move |x| {
            let c = &c;
            x + c.0
        })
        .cycle();
    for _ in 0..5 {
        println!("{:?}", cycled.next());
    }
    drop(cycled);
    println!("cycle gone");
    let e = Loud(5);
    let taken: Vec<u32> = vec![10, 20]
        .into_iter()
        .map(move |x| {
            let e = &e;
            x + e.0
        })
        .cycle()
        .take(5)
        .collect();
    println!("{taken:?}");

    let d = Loud(6);
    let caught = catch_unwind(move || {
        let mut cycled = vec![10]
            .into_iter()
            .map(move |x| {
                let d = &d;
                x + d.0
            })
            .cycle();
        println!("{:?}", cycled.next());
        PANIC_DROP_OF.with(|p| p.set(6));
        println!("{:?}", cycled.next());
        println!("unreached");
    });
    println!("caught {}", caught.is_err());
}
