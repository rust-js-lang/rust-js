// A thread-local's initializer of an `if` chain is one nested conditional,
// whatever its arms, a function or a closure among them, as React DOM's
// scheduleMicrotask is written; one whose arm does more than give its value
// is an arrow called in place still, and each test runs once, in order, as
// the log printed last shows.

use std::cell::RefCell;

fn double(x: u32) -> u32 {
    x * 2
}

fn triple(x: u32) -> u32 {
    x * 3
}

thread_local! {
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn level(at: &'static str) -> u32 {
    LOG.with_borrow_mut(|log| log.push(at));
    2
}

thread_local! {
    static PICK: fn(u32) -> u32 = if level("pick double") > 2 {
        double
    } else if level("pick closure") > 1 {
        |x| {
            let y = x + 1;
            y * 10
        }
    } else {
        triple
    };
    static NAME: String = if level("name big") > 5 {
        "big".to_owned()
    } else if level("name level") > 1 {
        if level("name two") == 2 {
            format!("level {}", level("name format"))
        } else {
            "odd".to_owned()
        }
    } else {
        String::new()
    };
    static WORKED: u32 = if level("worked") > 1 {
        let n = level("worked n");
        n * n + n
    } else {
        0
    };
}

fn main() {
    println!("{}", PICK.with(|f| *f)(4));
    println!("{}", NAME.with(|n| n.clone()));
    println!("{}", WORKED.with(|w| *w));
    LOG.with_borrow(|log| println!("{log:?}"));
}
