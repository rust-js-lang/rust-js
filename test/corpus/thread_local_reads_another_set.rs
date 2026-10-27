// A thread-local whose initializer reads what may change after its module
// loads is made where it's first used, as Rust makes it: another
// thread-local, but one made before it that holds what can't change; a
// `static mut`; or an atomic `static`. So is one whose initializer prints,
// gives std a closure that does, or may panic (ADR 0037, amended; #376).
// One that reads a `static` that can't change, `BASE`, or a thread-local
// of a `fn` made before it and never changed, `PICKED`, is made as its
// module loads.

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, Ordering};

static mut LIMIT: u32 = 1;
static HITS: AtomicU32 = AtomicU32::new(0);
static BASE: u32 = 5;

fn double(n: u32) -> u32 {
    n * 2
}

thread_local! {
    static TRIES: Cell<u32> = const { Cell::new(0) };
    static NAMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static MADE: Option<u32> = if TRIES.get() == 0 { Some(1) } else { None };
    static TAKEN: Vec<String> = NAMES.take();
    static LIMITED: u32 = unsafe { LIMIT } * 10;
    static COUNTED: u32 = HITS.load(Ordering::Relaxed) + 1;
    static BASED: u32 = BASE * 2;
    static PRINTED: u32 = {
        println!("printed");
        3
    };
    static MAPPED: Option<u32> = Some(2).map(|n| {
        println!("mapped {n}");
        n * 2
    });
    // Reads `PRINTED`, made where it's first used, so it is too.
    static AFTER_PRINTED: u32 = PRINTED.with(|p| *p) + 1;
    // Reads `LATER`, made after it, so it's made where it's first used.
    static BEFORE_LATER: u32 = LATER.with(|l| *l) + 1;
    static LATER: u32 = 7;
    static PICKED: fn(u32) -> u32 = double;
    static APPLIED: u32 = PICKED.with(|f| f(4));
    // Indexes past its end, so it's made where it's first used, where Rust
    // panics.
    static FIRST: u32 = [1, 2][BASE as usize];
}

fn main() {
    println!("main");
    TRIES.set(3);
    NAMES.with_borrow_mut(|names| names.push("a".to_string()));
    unsafe { LIMIT = 4 };
    HITS.fetch_add(6, Ordering::Relaxed);
    println!("{:?}", MADE.with(|m| *m));
    TAKEN.with(|taken| println!("{taken:?}"));
    NAMES.with_borrow(|names| println!("{}", names.len()));
    println!("{}", LIMITED.with(|l| *l));
    println!("{}", COUNTED.with(|c| *c));
    println!("{}", BASED.with(|b| *b));
    println!("{}", AFTER_PRINTED.with(|a| *a));
    println!("{:?}", MAPPED.with(|m| *m));
    println!("{}", BEFORE_LATER.with(|b| *b));
    println!("{}", APPLIED.with(|a| *a));
    println!("{}", std::panic::catch_unwind(|| FIRST.with(|f| *f)).is_err());
}
