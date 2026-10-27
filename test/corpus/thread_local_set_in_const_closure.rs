// A thread-local that's its module's variable, set only in a closure a
// `const` is made of, a module's or an impl's, as React's DEV dispatchers
// set `currentHookNameInDev` in theirs: `let`, as a function's set makes
// it, never a `const` reassigned, and read whole, `tries`, in an impl's
// `const` too (ADR 0270, amended).
use std::cell::{Cell, RefCell};

thread_local! {
    static TRIES: Cell<u32> = const { Cell::new(0) };
    static NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static LEVEL: Cell<u32> = const { Cell::new(0) };
}

struct Counter;

impl Counter {
    const BUMP: fn() = || TRIES.set(TRIES.get() + 1);
    const NAMED: fn(&str) = |name| NAME.set(name.to_string());
    // Set nowhere else.
    const DEEPER: fn() = || DEPTH.set(DEPTH.get() + 2);
}

const BUMP_TWICE: fn() = || {
    TRIES.set(TRIES.get() + 1);
    TRIES.set(TRIES.get() + 1);
};

// Set nowhere else either.
const LEVEL_UP: fn() = || LEVEL.set(LEVEL.get() + 1);

const SEE: fn(u32) = |n| SEEN.set(vec![n, n + 1]);

struct Dispatcher {
    use_name: fn(&str) -> usize,
}

thread_local! {
    static DISPATCHER: Option<Dispatcher> = Some(Dispatcher {
        use_name: |name| {
            NAME.set(name.to_uppercase());
            NAME.with_borrow(|n| n.len())
        },
    });
}

fn main() {
    (Counter::BUMP)();
    BUMP_TWICE();
    println!("tries {}", TRIES.get());
    (Counter::DEEPER)();
    println!("depth {}", DEPTH.get());
    LEVEL_UP();
    println!("level {}", LEVEL.get());
    (Counter::NAMED)("use_state");
    println!("name {}", NAME.with_borrow(|n| n.clone()));
    SEE(4);
    println!("seen {:?}", SEEN.with_borrow(|s| s.clone()));
    let len = DISPATCHER.with(|d| d.as_ref().map(|d| (d.use_name)("use_memo")));
    println!("len {len:?}, name {}", NAME.with_borrow(|n| n.clone()));
}
