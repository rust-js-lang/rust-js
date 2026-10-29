// A `match` of strings whose arms include a name compared, in its guard,
// with a thread-local or a `static`, read with no call: React's
// SimpleEventPlugin, `name if ANIMATION_END.with(|e| e == name) => ..`,
// upstream's `case ANIMATION_END:` (rd113, the user's decision). A JS
// `switch` evaluates each case in order until one matches, as Rust tries
// each guard, so a thread-local that changes between matches is read
// where Rust reads it. A guard with a call stays the guard it is.
use std::cell::{Cell, RefCell};

thread_local! {
    static ANIMATION_END: String = String::from("animationend");
    static CURRENT: RefCell<String> = RefCell::new(String::from("focusin"));
    static LIMIT: Cell<i32> = const { Cell::new(3) };
}

static FIXED: &str = "fixed";

fn on(log: &RefCell<Vec<String>>, name: &str, holds: bool) -> bool {
    log.borrow_mut().push(format!("{name}:{holds}"));
    holds
}

fn ctor(name: &str, n: i32, log: &RefCell<Vec<String>>) -> i32 {
    let mut ctor = 0;
    match name {
        // Remove unwanted keypresses.
        "keypress" if on(log, "code", n == 1) => return -1,
        "keypress" | "keydown" => ctor = 1,
        name if CURRENT.with_borrow(|current| current == name) => ctor = 6,
        "focusin" => ctor = 2,
        name if ANIMATION_END.with(|event_name| event_name == name) => ctor = 4,
        name if name == FIXED => ctor = 7,
        "scroll" | "scrollend" => ctor = 3,
        _ => {
            // Unknown event.
        }
    }
    ctor + n
}

fn limit(n: i32) -> &'static str {
    match n {
        0 => "zero",
        n if LIMIT.with(|limit| limit.get() == n) => "limit",
        1 => "one",
        _ => "other",
    }
}

/// A guarded arm whose failed guard goes past the arms after it, one of
/// which may match its value: an `if` chain, as Rust tries them.
fn past(name: &str, log: &RefCell<Vec<String>>) -> u32 {
    let mut n = 0;
    match name {
        "keypress" if on(log, "past", name.len() > 8) => n = 1,
        name if CURRENT.with_borrow(|current| current == name) => n = 2,
        "focusin" => n = 3,
        _ => {}
    }
    n
}

/// A guard whose value is a call's: an `if` still.
fn called(name: &str, log: &RefCell<Vec<String>>) -> u32 {
    match name {
        "a" => 1,
        name if on(log, name, name.len() == 3) => 2,
        "b" => 3,
        _ => 0,
    }
}

/// A thread-local's value a call makes, `e.trim()`: an `if` still.
fn trimmed(name: &str) -> u32 {
    match name {
        "a" => 1,
        name if ANIMATION_END.with(|event_name| event_name.trim() == name) => 2,
        "b" => 3,
        _ => 0,
    }
}

/// A parameter's value: an `if` still.
fn against(n: i32, limit: i32) -> u32 {
    match n {
        0 => 1,
        n if n == limit => 2,
        1 => 3,
        _ => 0,
    }
}

/// A cell's value, not a thread-local's: an `if` still.
fn against_cell(n: i32, cell: &Cell<i32>) -> u32 {
    match n {
        0 => 1,
        n if n == cell.get() => 2,
        1 => 3,
        _ => 0,
    }
}

/// A thread-local `Cell`'s `get()`, the key's own.
fn got(n: i32) -> &'static str {
    match n {
        0 => "zero",
        n if n == LIMIT.get() => "limit",
        2 => "two",
        _ => "other",
    }
}

fn main() {
    let log = RefCell::new(Vec::new());
    let names = ["keypress", "keydown", "focusin", "animationend", "fixed", "scroll", "other"];
    for name in names {
        println!("{name} {} {}", ctor(name, 1, &log), ctor(name, 2, &log));
    }
    CURRENT.with_borrow_mut(|current| *current = String::from("scroll"));
    for name in ["focusin", "scroll", "keydown"] {
        println!("{name} {}", ctor(name, 2, &log));
    }
    CURRENT.with_borrow_mut(|current| *current = String::from("keypress"));
    println!("keypress {} {}", ctor("keypress", 1, &log), ctor("keypress", 2, &log));
    println!("past {} {}", past("keypress", &log), past("focusin", &log));
    for n in 0..4 {
        println!("{n} {}", limit(n));
    }
    LIMIT.set(1);
    for n in 0..4 {
        println!("{n} {}", limit(n));
    }
    LIMIT.set(0);
    println!("0 {}", limit(0));
    for name in ["a", "abc", "b", "bc", "animationend"] {
        println!("{name} {} {}", called(name, &log), trimmed(name));
    }
    let cell = Cell::new(2);
    for n in 0..4 {
        println!("{n} {} {} {} {}", against(n, 1), against(n, 3), against_cell(n, &cell), got(n));
    }
    println!("{:?}", log.borrow());
}
