// A derived `Clone` of std's that clones a field of a type parameter's
// before another, `Chain`'s `a` before its `b`, `Zip`'s, `Peekable`'s
// `iter` before what it peeked, drops what it cloned where a later field's
// clone panics, as native Rust unwinds it. Std's derived `Clone` was taken
// to drop nothing, so it was given no drop of its `A` and dropped none
// (#1517).
use std::sync::atomic::{AtomicUsize, Ordering};
static CLONES: AtomicUsize = AtomicUsize::new(0);
static PANIC_AT: AtomicUsize = AtomicUsize::new(1);
struct Countdown(u32);
impl Iterator for Countdown {
    type Item = Item;
    fn next(&mut self) -> Option<Item> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(Item(self.0))
    }
}
fn tick() -> bool {
    CLONES.fetch_add(1, Ordering::Relaxed) == PANIC_AT.load(Ordering::Relaxed)
}
impl Clone for Countdown {
    fn clone(&self) -> Self {
        if tick() {
            panic!("clone");
        }
        Countdown(self.0 + 10)
    }
}
impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop countdown {}", self.0);
    }
}
#[derive(Debug)]
struct Item(u32);
impl Clone for Item {
    fn clone(&self) -> Self {
        if tick() {
            panic!("item clone");
        }
        Item(self.0 + 100)
    }
}
impl Drop for Item {
    fn drop(&mut self) {
        println!("drop item {}", self.0);
    }
}
fn chained() {
    let both = Countdown(2).chain(Countdown(1));
    CLONES.store(0, Ordering::Relaxed);
    PANIC_AT.store(1, Ordering::Relaxed);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let again: Vec<u32> = both.clone().map(|item| item.0).collect();
        println!("{again:?}");
    }));
    println!("chain caught {}", r.is_err());
    println!("{}", both.count());
}

fn main() {
    chained();
    let z = Countdown(2).zip(Countdown(1));
    CLONES.store(0, Ordering::Relaxed);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| z.clone().count()));
    println!("zip caught {}", r.is_err());
    let mut p = Countdown(3).peekable();
    println!("{:?}", p.peek());
    CLONES.store(0, Ordering::Relaxed);
    PANIC_AT.store(1, Ordering::Relaxed);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p.clone().count()));
    println!("peekable caught {}", r.is_err());
}
