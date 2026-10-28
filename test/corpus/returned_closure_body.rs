// A thread-local's `with_borrow_mut` whose closure returns early, the
// function's value: its body in place, each `return` the function's, as
// React's warnAboutUpdateOnNotYetMountedFiberInDEV writes `if (didWarn !==
// null) { if (didWarn.has(name)) { return; } didWarn.add(name); } else {
// didWarn = new Set([name]); }`, not a closure called where it's made, nor
// the variable a box (bx1, ADR 0581, amended).
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    static DID_WARN: RefCell<Option<HashSet<String>>> = const { RefCell::new(None) };
}

fn warn_once(name: &str) -> bool {
    DID_WARN.with_borrow_mut(|did_warn| {
        match did_warn {
            Some(did_warn) => {
                if did_warn.contains(name) {
                    return true;
                }
                did_warn.insert(name.to_string());
            }
            None => *did_warn = Some(HashSet::from([name.to_string()])),
        }
        false
    })
}

fn main() {
    for name in ["a", "b", "a", "c", "b"] {
        print!("{} ", warn_once(name));
    }
    println!("{}", DID_WARN.with_borrow(|w| w.as_ref().map_or(0, HashSet::len)));
}
