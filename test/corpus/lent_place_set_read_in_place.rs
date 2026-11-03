// A `Some` arm's binding into the place a nullable thread-local's
// `with_borrow_mut` lends, its body in place, is the module's variable
// itself where the arm never mentions the handle (rd723): React's
// warnAboutUpdateOnNotYetMountedFiberInDEV, `Some(did_warn) => { if
// did_warn.contains(..) { return true; } did_warn.insert(..); }`, is
// upstream's `if (didWarn != null) { if (didWarn.has(name)) { return; }
// didWarn.add(name); }`, no `const didWarn$1 = didWarn;`. The borrow is
// exclusive while the closure runs, so nothing else writes the place. Kept a
// copy: an arm that sets the place through the handle after its binding's
// last read, `*names = None;`, one that sets it from a copy of what it held,
// `*count = Some(n + 1);`, one with a closure reading the binding, and one
// whose test is more than that it's set, `Some(n) if n > limit`. Nor is what
// begins a branch that's not what the place holds, `let stamp = tick();`.
use std::cell::{Cell, RefCell};
use std::collections::HashSet;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static DID_WARN: RefCell<Option<HashSet<String>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NAMES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COUNT: RefCell<Option<u32>> = const { RefCell::new(None) };
    static TICKS: Cell<u32> = const { Cell::new(0) };
}

fn tick() -> u32 {
    let n = TICKS.get() + 1;
    TICKS.set(n);
    n
}

fn warn_once(name: &str) {
    let did_warn = DID_WARN.with_borrow_mut(|did_warn| {
        match did_warn {
            Some(did_warn) => {
                if did_warn.contains(name) {
                    return true;
                }
                did_warn.insert(name.to_string());
            }
            None => {
                *did_warn = Some(HashSet::from([name.to_string()]));
            }
        }
        false
    });
    if did_warn {
        return;
    }
    println!("warn {name}");
}

fn reset_when_full(name: &str) {
    let full = NAMES.with_borrow_mut(|names| {
        match names {
            Some(held) => {
                held.push(name.to_string());
                let len = held.len();
                if len > 3 {
                    *names = None;
                    return true;
                }
            }
            None => {
                *names = Some(vec![name.to_string()]);
            }
        }
        false
    });
    if full {
        return;
    }
    println!("kept {name}");
}

fn bump() {
    let bumped = COUNT.with_borrow_mut(|count| {
        match *count {
            Some(n) => {
                *count = Some(n + 1);
                if n > 1 {
                    return true;
                }
            }
            None => {
                *count = Some(0);
            }
        }
        false
    });
    if bumped {
        return;
    }
    println!("bumped {:?}", COUNT.with_borrow(|c| *c));
}

fn counted(name: &str) {
    let many = DID_WARN.with_borrow_mut(|did_warn| {
        match did_warn {
            Some(set) => {
                let count = || set.len();
                if count() > 3 {
                    return true;
                }
                set.insert(name.to_string());
            }
            None => {
                *did_warn = Some(HashSet::new());
            }
        }
        false
    });
    if many {
        return;
    }
    println!("counted {name}");
}

fn guarded(limit: u32) {
    let over = COUNT.with_borrow_mut(|count| {
        match *count {
            Some(n) if n > limit => {
                if n > 100 {
                    return true;
                }
            }
            _ => {
                *count = Some(limit + 1);
            }
        }
        false
    });
    if over {
        return;
    }
    println!("guarded {limit}");
}

fn stamped(name: &str) {
    let seen = DID_WARN.with_borrow_mut(|did_warn| {
        match did_warn {
            Some(_) => {
                let stamp = tick();
                if stamp > 2 {
                    return true;
                }
                println!("{name} {stamp} {stamp}");
            }
            None => {
                *did_warn = Some(HashSet::new());
            }
        }
        false
    });
    if seen {
        return;
    }
    println!("stamped {name}");
}

pub fn main() {
    warn_once("A");
    warn_once("A");
    warn_once("B");
    for name in ["a", "b", "c", "d", "e"] {
        reset_when_full(name);
    }
    for _ in 0..4 {
        bump();
    }
    for name in ["p", "q", "r"] {
        counted(name);
    }
    guarded(1);
    guarded(5);
    guarded(1);
    for name in ["s", "t", "u"] {
        stamped(name);
    }
}
