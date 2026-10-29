// A `match` of strings whose guarded arm falls to the next, which names
// its string too: React's SimpleEventPlugin, `"keypress" if
// char_code(event) == 0 => return None`, then `"keypress" | "keydown"`. A
// `switch` whose guarded case falls through, as of numbers (ADR 0905), and
// in a loop, the `if` chain Rust tries (#389b); a string pattern's value
// is its text, which tells two apart (rd112).
use std::cell::RefCell;

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
        "focusin" => ctor = 2,
        "scroll" | "scrollend" => ctor = 3,
        _ => {
            // Unknown event.
        }
    }
    ctor + n
}

fn scan(names: &[&str], log: &RefCell<Vec<String>>) -> u32 {
    let mut n = 0;
    for &name in names {
        match name {
            "stop" if on(log, "stop", n > 2) => break,
            "stop" | "go" => n += 2,
            "skip" => continue,
            _ => {}
        }
        n += 1;
    }
    n
}

fn main() {
    let log = RefCell::new(Vec::new());
    for name in ["keypress", "keydown", "focusin", "scroll", "scrollend", "click", ""] {
        for n in [0, 1] {
            let made = ctor(name, n, &log);
            println!("{name:?} {n}: {made} {:?}", log.take());
        }
    }
    for names in [&[][..], &["stop"], &["go", "stop"], &["go", "go", "stop", "x"], &["skip", "go", "stop"]] {
        let n = scan(names, &log);
        println!("{names:?}: {n} {:?}", log.take());
    }
}
