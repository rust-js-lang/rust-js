// A stage that discards an item, a `filter` it doesn't keep or the item a
// `take_while` stops at, drops it as its fold closure ends: std's closure
// has moved the accumulator into what it gives by then, so a panic of
// that drop leaks the accumulator (rustc #47949), from the front, the
// back and a `try_fold`, of a `Vec`'s items, a zip's and a borrowed
// slice's mapped, and in generic code (#651). Where the closure still
// holds it, a predicate's own panic or a `skip_while`'s skipped item, it's
// dropped as it unwinds.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.with(Cell::get) {
            BOOM.with(|boom| boom.set(0));
            panic!("drop");
        }
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..8).map(D).collect()
}

fn odd(d: &D) -> bool {
    d.0 % 2 == 1
}

fn add(mut acc: Acc, d: D) -> Acc {
    acc.0 += d.0;
    acc
}

fn try_add(mut acc: Acc, d: D) -> Result<Acc, u32> {
    acc.0 += d.0;
    Ok(acc)
}

fn sum_odd<I: Iterator<Item = D>>(items: I) -> u32 {
    items.filter(odd).fold(Acc(0), add).0
}

fn run(name: &str, boom: u32, case: fn() -> u32) {
    println!("{name}");
    BOOM.with(|b| b.set(boom));
    match catch_unwind(case) {
        Ok(n) => println!("ended {n}"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    run("filter fold", 4, || ds().into_iter().filter(odd).fold(Acc(0), add).0);
    run("filter rfold", 4, || ds().into_iter().filter(odd).rfold(Acc(0), add).0);
    run("filter try_fold", 4, || {
        let r = ds().into_iter().filter(odd).try_fold(Acc(0), try_add);
        r.map_or(0, |acc| acc.0)
    });
    run("zip filter try_fold", 4, || {
        let r = ds()
            .into_iter()
            .zip(0..7)
            .filter(|(d, _)| odd(d))
            .try_fold(Acc(0), |acc, (d, _)| try_add(acc, d));
        r.map_or(0, |acc| acc.0)
    });
    run("mapped slice filter fold", 4, || {
        let nums = [1, 2, 3, 4, 5];
        nums.iter().map(|&n| D(n)).filter(odd).fold(Acc(0), add).0
    });
    run("generic filter fold", 4, || sum_odd(ds().into_iter()));
    run("take_while fold", 4, || ds().into_iter().take_while(|d| d.0 < 4).fold(Acc(0), add).0);
    run("take_while try_fold", 4, || {
        let r = ds().into_iter().take_while(|d| d.0 < 4).try_fold(Acc(0), try_add);
        r.map_or(0, |acc| acc.0)
    });
    run("filter count", 4, || ds().into_iter().filter(odd).count() as u32);
    run("predicate panics", 0, || {
        let keep = |d: &D| {
            if d.0 == 4 {
                panic!("keep");
            }
            odd(d)
        };
        ds().into_iter().filter(keep).fold(Acc(0), add).0
    });
    run("skip_while try_fold", 2, || {
        let r = ds().into_iter().skip_while(|d| d.0 < 3).try_fold(Acc(0), try_add);
        r.map_or(0, |acc| acc.0)
    });
}
