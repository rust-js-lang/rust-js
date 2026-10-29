// A thread-local whose initializer reads what may change after its module
// loads is made as its module loads, as JS's `const x = f();` is, whatever
// it reads (ADR 1006, amended, the user's decision): another thread-local,
// a `static mut` or an atomic `static`; so is one whose initializer prints
// or gives std a closure that does, and one that reads another made after
// it, after that one. It differs from Rust only in when it runs, so `main`
// first reads each, in the order they're made, and then changes what they
// read (#376).

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
    // Reads `PRINTED`, made before it.
    static AFTER_PRINTED: u32 = PRINTED.with(|p| *p) + 1;
    // Reads `LATER`, written after it, so it's made after that one.
    static BEFORE_LATER: u32 = LATER.with(|l| *l) + 1;
    static LATER: u32 = 7;
    static PICKED: fn(u32) -> u32 = double;
    static APPLIED: u32 = PICKED.with(|f| f(4));
}

// A thread-local of another module's, made first as that module loads
// first, is read as its module loads; one of a module that imports its own
// back, in an import cycle, which may load after it, where it's first used.
mod flags {
    thread_local! {
        pub static ON: bool = true;
    }
}

thread_local! {
    static USES_FLAG: bool = flags::ON.with(|&on| on) && BASE > 1;
}

mod marks {
    thread_local! {
        pub static LIMIT: u32 = 4;
    }

    pub fn used() -> u32 {
        super::uses::read()
    }
}

mod uses {
    thread_local! {
        static LIMITED: u32 = super::marks::LIMIT.with(|&limit| limit) + 1;
    }

    pub fn read() -> u32 {
        LIMITED.with(|&limited| limited)
    }
}

fn main() {
    let made_at_start = MADE.with(|m| *m);
    let taken_at_start = TAKEN.with(Clone::clone);
    let limited = LIMITED.with(|l| *l);
    let counted = COUNTED.with(|c| *c);
    let based = BASED.with(|b| *b);
    let after_printed = AFTER_PRINTED.with(|a| *a);
    let mapped_at_start = MAPPED.with(|m| *m);
    let before_later = BEFORE_LATER.with(|b| *b);
    let applied = APPLIED.with(|a| *a);
    println!("main");
    println!("{} {}", USES_FLAG.with(|&uses| uses), marks::used());
    TRIES.set(3);
    NAMES.with_borrow_mut(|names| names.push("a".to_string()));
    unsafe { LIMIT = 4 };
    HITS.fetch_add(6, Ordering::Relaxed);
    println!("{made_at_start:?} {:?}", MADE.with(|m| *m));
    println!("{taken_at_start:?}");
    NAMES.with_borrow(|names| println!("{}", names.len()));
    println!("{limited} {}", LIMITED.with(|l| *l));
    println!("{counted}");
    println!("{based}");
    println!("{after_printed}");
    println!("{mapped_at_start:?}");
    println!("{before_later}");
    println!("{applied}");
    // A closure's parameter named as the module's `tries` is, stays apart
    // from it.
    let tries: Vec<u32> = [1, 2].iter().map(|tries| tries + 1).collect();
    println!("{tries:?}");
}
