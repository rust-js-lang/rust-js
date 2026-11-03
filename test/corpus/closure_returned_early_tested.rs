// A closure called where it's made, a thread-local's `with_borrow_mut`,
// that returns `true` early and `false` at its end, whose value only the
// `if` after it tests, is its body in place, each `return` what that `if`
// does (#571): React's warnAboutUpdateOnNotYetMountedFiberInDEV, `let
// did_warn = DID_WARN.with_borrow_mut(|did_warn| { .. return true; ..
// false }); if did_warn { return; }`, is upstream's `if
// (didWarn.has(componentName)) { return; }`, of a nullable thread-local
// that's its module's variable too. Tested negated, `if !fresh`, of a
// function that returns a value, and of a branch that reads the function's
// `n` beside a block's own, `n$1`. Not tested so: a flag read again, a
// branch that does more than return, and a value not a `bool` written,
// each its body in place setting its `let`, `fresh = false;` (#591).
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static DID_WARN: RefCell<Option<HashSet<String>>> = const { RefCell::new(None) };
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static BIG: RefCell<Vec<u32>> = RefCell::new(vec![500]);
}

fn warn(component_name: &str) {
    let did_warn = DID_WARN.with_borrow_mut(|did_warn| {
        match did_warn {
            Some(did_warn) => {
                if did_warn.contains(component_name) {
                    return true;
                }
                did_warn.insert(component_name.to_string());
            }
            None => {
                *did_warn = Some(HashSet::from([component_name.to_string()]));
            }
        }
        false
    });
    if did_warn {
        return;
    }
    println!("warn {component_name}");
}

fn first_time(n: u32) -> u32 {
    let fresh = SEEN.with_borrow_mut(|seen| {
        if seen.contains(&n) {
            return false;
        }
        seen.push(n);
        true
    });
    if !fresh {
        return 0;
    }
    n * 10
}

fn read_again(n: u32) -> String {
    let fresh = SEEN.with_borrow_mut(|seen| {
        if seen.contains(&n) {
            return false;
        }
        seen.push(n);
        true
    });
    if fresh {
        return format!("fresh {fresh}");
    }
    format!("seen {fresh}")
}

fn logged(n: u32) {
    let seen_before = SEEN.with_borrow_mut(|seen| {
        if seen.contains(&n) {
            return true;
        }
        seen.push(n);
        false
    });
    if seen_before {
        println!("seen {n}");
        return;
    }
    println!("new {n}");
}

fn counted(n: u32) -> usize {
    let count = SEEN.with_borrow_mut(|seen| {
        if seen.contains(&n) {
            return 0;
        }
        seen.push(n);
        seen.len()
    });
    if count == 0 {
        return 99;
    }
    count
}

fn shadowed(n: u32) -> u32 {
    let big = BIG.with_borrow_mut(|big| {
        if !big.is_empty() {
            let n = big[0];
            if n > 100 {
                return true;
            }
        }
        big.push(1);
        false
    });
    if big {
        return n;
    }
    n + 1
}

fn warned() -> usize {
    DID_WARN.with_borrow(|did_warn| did_warn.as_ref().map_or(0, HashSet::len))
}

pub fn main() {
    warn("a");
    warn("b");
    warn("a");
    println!("{}", warned());
    println!("{} {} {}", first_time(1), first_time(2), first_time(1));
    println!("{} {}", read_again(3), read_again(3));
    logged(4);
    logged(4);
    println!("{} {}", counted(5), counted(5));
    println!("{}", shadowed(7));
}
