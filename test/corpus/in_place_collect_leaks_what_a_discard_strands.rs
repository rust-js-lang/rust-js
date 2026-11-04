// A `collect()` into a `Vec` that std makes in place, of a `filter` or a
// `take_while` of a `Vec`'s items: its sink, what it collected, is the
// `try_fold`'s accumulator, which the stage's closure has moved into what
// it gives when it drops the item it discards; a panic of that drop leaks
// what it collected, as rustc leaks it (rust-lang/rust#47949), and drops
// what's left. Where a stage panics, a `map`'s, it's dropped (#681).
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

fn ds() -> Vec<D> {
    (1..8).map(D).collect()
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
    run("filter", 4, || ds().into_iter().filter(|d| d.0 != 4).collect::<Vec<_>>().len());
    run("take_while", 4, || ds().into_iter().take_while(|d| d.0 < 4).collect::<Vec<_>>().len());
    run("map filter", 4, || {
        ds().into_iter().map(|d| d).filter(|d| d.0 % 2 == 1).collect::<Vec<_>>().len()
    });
    run("map panics", 0, || {
        ds().into_iter()
            .map(|d| if d.0 == 4 { panic!("map") } else { d })
            .collect::<Vec<_>>()
            .len()
    });
    run("filter ends", 0, || ds().into_iter().filter(|d| d.0 != 4).collect::<Vec<_>>().len());
}
