// A `match` of constants that's its function's value, whose guarded arm
// just before a last `_` that does something goes on to it where the
// guard fails: React's `describeFiber`, a `switch` whose guarded case
// falls through to its `default` (ADR 0636, amended). Each guard runs
// where Rust runs it, once its constant matched.
use std::cell::RefCell;

const HOST: u32 = 5;
const FUNCTION: u32 = 0;
const MEMO: u32 = 15;
const LAZY: u32 = 16;
const VIEW_TRANSITION: u32 = 30;

/// A guard that says it ran.
fn on(log: &RefCell<Vec<String>>, name: &str, value: bool) -> bool {
    log.borrow_mut().push(name.to_string());
    value
}

fn frame(tag: u32, child: bool, view_transitions: bool, log: &RefCell<Vec<String>>) -> String {
    match tag {
        HOST => "host".to_string(),
        LAZY => "Lazy".to_string(),
        FUNCTION | MEMO => {
            if child {
                return "Fallback".to_string();
            }
            "function".to_string()
        }
        VIEW_TRANSITION if on(log, "frame", view_transitions) => "ViewTransition".to_string(),
        _ => format!("<{tag}>"),
    }
}

/// Of a statement `match`, its `_` named, more after it.
fn counted(tags: &[u32], log: &RefCell<Vec<String>>) -> u32 {
    let mut total = 0;
    for &tag in tags {
        match tag {
            HOST => total += 1,
            LAZY => total += 300,
            VIEW_TRANSITION if on(log, "even", total % 2 == 0) => total += 20,
            other => total += other * 1000,
        }
        total += 1;
    }
    total
}

fn main() {
    let log = RefCell::new(Vec::new());
    for tag in [0, 5, 15, 16, 30, 99] {
        for child in [false, true] {
            for view_transitions in [false, true] {
                let frame = frame(tag, child, view_transitions, &log);
                println!("{tag} {child} {view_transitions}: {frame} {:?}", log.take());
            }
        }
    }
    println!("{} {:?}", counted(&[5, 30, 30, 16, 30, 7, 5, 30], &log), log.take());
}
