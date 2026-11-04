// What an adapter passes over is dropped as it passes it, and a drop that
// panics there unwinds through what consumes it: a `step_by`'s `nth`, of a
// `Vec`'s, drops what it passes as one slice, the rest of the slice after a
// panicking item, then the fold's accumulator, then what's left (#668); a
// `skip`'s and a `step_by` of a `step_by`'s alike, folded,
// summed into what has a destructor, collected, counted and `try_fold`ed,
// from the back too, though nothing
// else in the chain may panic, the closure only moving what it's given.
use std::cell::Cell;
use std::iter::Sum;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

struct Total(u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop total {}", self.0);
    }
}

impl Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Self {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

fn seven() -> Vec<D> {
    (1..=7).map(D).collect()
}

fn add(mut a: Acc, d: D) -> Acc {
    a.0 += d.0;
    a
}

fn kept(mut v: Vec<D>, d: D) -> Vec<D> {
    v.push(d);
    v
}

fn run(name: &str, boom: u32, f: impl FnOnce() -> u32) {
    println!("-- {name}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("step_by step_by fold", 3, || {
        seven().into_iter().step_by(2).step_by(2).fold(Acc(0), add).0
    });
    run("step_by fold", 2, || seven().into_iter().step_by(3).fold(Acc(0), add).0);
    run("step_by fold kept", 2, || {
        seven().into_iter().step_by(3).fold(Vec::new(), kept).len() as u32
    });
    run("step_by step_by sum", 3, || {
        seven().into_iter().step_by(2).step_by(2).sum::<Total>().0
    });
    run("step_by collect", 5, || {
        seven().into_iter().step_by(3).collect::<Vec<_>>().len() as u32
    });
    run("step_by count", 2, || seven().into_iter().step_by(3).count() as u32);
    run("skip fold", 1, || seven().into_iter().skip(3).fold(Acc(0), add).0);
    run("skip collect", 2, || {
        seven().into_iter().skip(3).collect::<Vec<_>>().len() as u32
    });
    run("skip map collect", 2, || seven().into_iter().skip(3).map(|d| d.0).sum());
    run("skip step_by try_fold", 3, || {
        let r: Result<Acc, ()> = (seven().into_iter().skip(1).step_by(2)).try_fold(Acc(0), |a, d| Ok(add(a, d)));
        r.map_or(0, |a| a.0)
    });
    run("rev step_by step_by fold", 5, || {
        (seven().into_iter().rev().step_by(2).step_by(2)).fold(Acc(0), add).0
    });
    run("no panic", 0, || {
        seven().into_iter().step_by(2).step_by(2).fold(Acc(0), add).0
    });
}
