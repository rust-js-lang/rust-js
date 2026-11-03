// A closure called where it's made whose value a `let` holds, which returns
// a value before its end, is its body in place: `let first;`, each `return
// v` setting it, `first = v;`, what follows the `return`'s `if` its `else`
// (#591). A nullable thread-local's `with_borrow_mut(|names| { if let
// Some(names) = names { ..; return first; } .. })` reads and writes the
// module's variable, not a getter and setter handle on it. A name the body
// declares in a block of its own that's the `let`'s is named apart. Kept a
// call: a `return` in a loop, which setting the value wouldn't leave.
use std::cell::{Cell, RefCell};

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NAMES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static LIMIT: RefCell<Option<u32>> = const { RefCell::new(None) };
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

fn first_name(name: &str) -> String {
    let first = NAMES.with_borrow_mut(|names| {
        if let Some(names) = names {
            let first = names[0].clone();
            names.push(name.to_string());
            return first;
        }
        *names = Some(vec![name.to_string()]);
        name.to_string()
    });
    println!("first {first} of {name}");
    first
}

fn limited(n: u32) -> u32 {
    let mut seen = LIMIT.with_borrow_mut(|limit| {
        let seen = n * 2;
        if let Some(limit) = limit
            && seen > *limit
        {
            return *limit;
        }
        *limit = Some(seen);
        seen
    });
    if seen > 10 {
        seen = 10;
    }
    println!("limited {n}: {seen}");
    seen
}

fn counted() -> u32 {
    let n = COUNT.with(|count| {
        if count.get() > 2 {
            return 0;
        }
        count.set(count.get() + 1);
        count.get()
    });
    println!("counted {n}");
    n
}

fn found(names: &[&str], wanted: &str) -> usize {
    let at = NAMES.with_borrow_mut(|held| {
        for (i, name) in names.iter().enumerate() {
            if *name == wanted {
                return i;
            }
        }
        held.as_ref().map_or(0, Vec::len)
    });
    println!("found {wanted} at {at}");
    at
}

pub fn main() {
    for name in ["a", "b", "c"] {
        first_name(name);
    }
    for n in [3, 9, 2, 1] {
        limited(n);
    }
    for _ in 0..4 {
        counted();
    }
    found(&["x", "y"], "y");
    found(&["x", "y"], "z");
}
