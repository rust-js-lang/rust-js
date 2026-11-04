// A thread-local is made where it's first used, as Rust makes it, where
// making it as its module loads would show (#554, the user's decision, ADR
// 1006, amended): what its initializer runs prints or panics, reads state
// something other than the loading initializers writes, `LEN`, which `main`
// sets before the first use, or writes state that's read after the modules
// load, `COUNT`; so one that reads what a body only it ran sets, `LOGS`, or
// what a closure kept and called later sets, `BUMPS`, or a struct's cell,
// `SEEN`. One that only makes a value, of a `const { .. }` or a
// binding's call, `Math.max`, or reads and writes what only the loading
// initializers touch, a cache only they fill and read, is made as its
// module loads.
// A lazy `#[rust_js::nullable]` one stores `null` for `None`, however it's
// written.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

fn none_of(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

fn made() -> Option<u32> {
    println!("made");
    none_of(LEN.with(|l| l.get()))
}

fn len_now() -> u32 {
    LEN.with(|l| l.get())
}

fn count() -> u32 {
    let n = COUNT.get() + 1;
    COUNT.set(n);
    n
}

fn checked(n: u32) -> u32 {
    if n == 0 {
        panic!("no length");
    }
    n
}

// Only the loading initializers call it: the cache is theirs alone, filled
// in a closure `with` runs there.
fn prefixed(name: &str) -> String {
    if let Some(hit) = CACHE.with_borrow(|cache| cache.get(name).cloned()) {
        return hit;
    }
    VENDOR.with(|vendor| {
        let made = format!("{vendor}{name}");
        CACHE.with_borrow_mut(|cache| cache.insert(name.to_string(), made.clone()));
        made
    })
}

// Called only by `LOGGED`'s initializer, which prints, so after the modules
// load: what it sets, `LOGS`, is set then.
fn log() -> u32 {
    println!("logged");
    LOGS.set(LOGS.get() + 1);
    1
}

// What it gives is kept, called by `main`: what it sets is set after the
// modules load.
fn make_bump() -> Box<dyn Fn()> {
    Box::new(|| BUMPS.set(BUMPS.get() + 1))
}

struct Seen {
    count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
fn max(a: f64, b: f64) -> f64 {
    a.max(b)
}

thread_local! {
    static LEN: Cell<u32> = const { Cell::new(1) };
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static CACHE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    static VENDOR: String = String::from("webkit");
    static LOGS: Cell<u32> = const { Cell::new(0) };
    static BUMPS: Cell<u32> = const { Cell::new(0) };
    static SEEN: Seen = const { Seen { count: Cell::new(0) } };
    // Made where it's first used.
    static LAZY: RefCell<Option<u32>> = RefCell::new(made());
    static PRINTED: u32 = {
        println!("printed");
        3
    };
    static LONGER: u32 = len_now() * 2;
    static DIRECT: u32 = LEN.get() + 1;
    static COUNTED: u32 = count();
    static CHECKED: u32 = checked(2);
    static LOGGED: u32 = log();
    static LOGS_AT_FIRST: u32 = LOGS.get();
    static BUMPS_AT_FIRST: u32 = BUMPS.get();
    static SEEN_AT_FIRST: u32 = SEEN.with(|seen| seen.count.get());
    #[cfg_attr(rust_js, rust_js::nullable)]
    static FOUND: RefCell<Option<u32>> = RefCell::new(none_of(len_now()));
    #[cfg_attr(rust_js, rust_js::nullable)]
    static SET_FIRST: RefCell<Option<u32>> = RefCell::new(none_of(len_now()));
    // Made as its module loads.
    static FIXED: Cell<u32> = const { Cell::new(7) };
    static BIGGER: f64 = max(1.0, 2.0);
    static BUMP: Box<dyn Fn()> = make_bump();
    static END: String = prefixed("end");
    static START: String = prefixed("start");
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn found() -> Option<u32> {
    FOUND.with_borrow(|f| *f)
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn absent(x: Option<u32>) -> bool {
    x.is_none()
}

fn main() {
    println!("start");
    LEN.set(5);
    println!("{:?}", LAZY.with_borrow(Clone::clone));
    println!("{} {}", LONGER.with(|l| *l), DIRECT.with(|d| *d));
    println!("{}", COUNT.get());
    println!("{}", COUNTED.with(|c| *c));
    println!("{}", COUNT.get());
    println!("{}", PRINTED.with(|p| *p));
    println!("{}", CHECKED.with(|c| *c));
    println!("{} {}", LOGGED.with(|l| *l), LOGS_AT_FIRST.with(|l| *l));
    BUMP.with(|bump| bump());
    SEEN.with(|seen| seen.count.set(2));
    println!("{} {}", BUMPS_AT_FIRST.with(|b| *b), SEEN_AT_FIRST.with(|s| *s));
    println!("{:?} {}", found(), absent(found()));
    FOUND.set(None);
    println!("{} {}", FOUND.with_borrow(Option::is_none), absent(found()));
    FOUND.set(Some(2));
    println!("{:?}", FOUND.replace(None));
    println!("{} {}", FOUND.with_borrow(Option::is_none), absent(found()));
    FOUND.set(Some(3));
    FOUND.with_borrow_mut(|f| *f = None);
    println!("{} {}", FOUND.with_borrow(Option::is_none), absent(found()));
    // `set` before its first use, which makes it of what it's given.
    SET_FIRST.set(Some(4));
    SET_FIRST.with_borrow_mut(|f| *f = None);
    println!("{}", absent(SET_FIRST.with_borrow(|f| *f)));
    FIXED.set(FIXED.get() + 1);
    println!("{} {}", FIXED.get(), BIGGER.with(|b| *b));
    println!("{} {}", END.with(Clone::clone), START.with(Clone::clone));
}
