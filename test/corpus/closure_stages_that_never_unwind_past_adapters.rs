// A closure stage that can't panic doesn't make an adapter's own work one
// that can't (#701, ADR 1089): a `filter()` drops the item its closure
// rejects, a `take_while()` the one it refuses, a `fuse()` and a `chain()`
// the iterator they've exhausted, and a `flat_map()` its inner one, and
// each drop here panics once, of the value `BOOM` names, so a collect
// drops what it collected and what's left as std's unwinds.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

fn boom(n: u32) -> bool {
    let hit = n != 0 && n == BOOM.with(Cell::get);
    if hit {
        BOOM.with(|boom| boom.set(0));
    }
    hit
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if boom(self.0) {
            panic!("drop");
        }
    }
}

// The crate's iterator, made by its constructor alone, which gives one
// item, and whose drop panics.
struct Tick(u32);

impl Drop for Tick {
    fn drop(&mut self) {
        println!("drop tick {}", self.0);
        if boom(self.0) {
            panic!("drop");
        }
    }
}

impl Iterator for Tick {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 >= 100 {
            None
        } else {
            self.0 += 100;
            Some(self.0)
        }
    }
}

fn ds() -> Vec<D> {
    (1..6).map(D).collect()
}

fn run(name: &str, boom: u32, case: fn() -> usize) {
    println!("{name}");
    BOOM.with(|b| b.set(boom));
    match catch_unwind(case) {
        Ok(n) => println!("ended {n}"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    run("filter", 3, || ds().into_iter().filter(|d| d.0 != 3).collect::<Vec<_>>().len());
    run("take_while", 3, || ds().into_iter().take_while(|d| d.0 < 3).collect::<Vec<_>>().len());
    run("fuse", 107, || Tick(7).fuse().map(|n| D(n + 50)).collect::<Vec<_>>().len());
    run("chain", 107, || Tick(7).map(|n| D(n + 50)).chain(ds()).collect::<Vec<_>>().len());
    run("flat_map", 107, || (7..9).flat_map(Tick).map(|n| D(n + 50)).collect::<Vec<_>>().len());
    run("none panics", 0, || {
        let kept = ds().into_iter().filter(|d| d.0 % 2 == 1).collect::<Vec<_>>().len();
        kept + Tick(7).fuse().map(|n| D(n + 50)).chain(ds()).collect::<Vec<_>>().len()
    });
}
