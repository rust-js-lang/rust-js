// What std's iterators do themselves as they're stepped, besides their
// stages' closures, may panic, and a collect it stops drops what it
// collected and what's left, as std's unwinds (#707–#711): a `zip()` drops
// the item it took as its other side ends, or those it trims from the
// longer side's back; a `chain()` or a `flatten()` drops a part it has
// exhausted; a `cloned()`, a `cycle()` or a `repeat()` clones. Each drop
// here panics once, of the value `BOOM` names, and a clone at the count
// `CLONES` names.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
    static CLONES: Cell<u32> = const { Cell::new(0) };
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

// What a drop that never panics drops.
struct Quiet(u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        println!("drop quiet {}", self.0);
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        let left = CLONES.with(Cell::get);
        CLONES.with(|clones| clones.set(left.saturating_sub(1)));
        if left == 1 {
            panic!("clone");
        }
        D(self.0 + 100)
    }
}

// The crate's iterator, whose `next` never panics, and whose drop and
// clone do.
#[derive(Clone)]
struct Once {
    _held: D,
    left: u32,
}

impl Iterator for Once {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            None
        } else {
            self.left -= 1;
            Some(self.left)
        }
    }
}

// The crate's iterator, which holds nothing to drop, and whose clone
// panics.
struct Twice(u32);

impl Clone for Twice {
    fn clone(&self) -> Twice {
        println!("clone twice");
        let left = CLONES.with(Cell::get);
        CLONES.with(|clones| clones.set(left.saturating_sub(1)));
        if left == 1 {
            panic!("clone");
        }
        Twice(self.0)
    }
}

impl Iterator for Twice {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

fn ds() -> Vec<D> {
    (1..6).map(D).collect()
}

fn once(held: u32) -> Once {
    Once {
        _held: D(held),
        left: 1,
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

fn run(name: &str, (boom, clones): (u32, u32), case: fn() -> usize) {
    println!("{name}");
    BOOM.with(|b| b.set(boom));
    CLONES.with(|c| c.set(clones));
    match catch_unwind(case) {
        Ok(n) => println!("ended {n}"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    run("zip", (3, 0), || ds().into_iter().zip(0..2).collect::<Vec<_>>().len());
    run("zip from the back", (5, 0), || {
        ds().into_iter().zip(vec![D(20), D(21)]).rev().collect::<Vec<_>>().len()
    });
    run("zip trimmed of its other side", (4, 0), || {
        vec![Quiet(20), Quiet(21)].into_iter().zip(ds()).rev().collect::<Vec<_>>().len()
    });
    run("cloned", (0, 2), || {
        let v = ds();
        v.iter().cloned().collect::<Vec<_>>().len()
    });
    run("cycle", (0, 2), || once(5).cycle().take(3).collect::<Vec<_>>().len());
    run("cycle of what holds nothing", (0, 2), || {
        Twice(2).cycle().take(5).map(D).collect::<Vec<_>>().len()
    });
    run("repeat", (0, 2), || std::iter::repeat(D(30)).take(3).collect::<Vec<_>>().len());
    run("repeat_n", (0, 2), || std::iter::repeat_n(D(30), 3).collect::<Vec<_>>().len());
    run("chain", (9, 0), || once(9).chain(once(8)).collect::<Vec<_>>().len());
    run("flatten", (7, 0), || {
        vec![once(7), once(6)].into_iter().flatten().collect::<Vec<_>>().len()
    });
    run("flatten of a map", (107, 0), || (7..9).map(Tick).flatten().map(D).collect::<Vec<_>>().len());
    run("flat_map", (107, 0), || (7..9).flat_map(Tick).map(D).collect::<Vec<_>>().len());
    run("none panics", (0, 0), || {
        let v = ds();
        let zipped = v.iter().cloned().zip(once(7).chain(once(8))).count();
        let repeated = std::iter::repeat(D(31)).take(2).collect::<Vec<_>>().len();
        zipped + repeated + vec![once(6)].into_iter().flatten().count()
    });
}
