// A `match` of constants with two guarded arms, one whose failed guard
// goes on to the next arm, which names its constant, and one whose failed
// guard goes on to nothing: React's `getOwnerStackByFiberInDev`, a
// `switch` whose guarded cases fall through to the next case or break
// (ADRs 0636, 0905, amended). Each guard runs where Rust runs it, once its
// constant matched, and only until one holds.
use std::cell::RefCell;

const HOST: u32 = 5;
const FUNCTION: u32 = 0;
const MEMO: u32 = 15;
const FORWARD_REF: u32 = 11;
const LAZY: u32 = 16;
const VIEW_TRANSITION: u32 = 30;

/// A guard that says it ran.
fn on(log: &RefCell<Vec<String>>, name: &str, value: bool) -> bool {
    log.borrow_mut().push(name.to_string());
    value
}

/// What's built of a statement `match`, more after it.
fn described(tag: u32, owned: bool, flags: [bool; 2], log: &RefCell<Vec<String>>) -> String {
    let [view_transitions, prefixed] = flags;
    let mut info = String::new();
    if prefixed {
        info += "> ";
    }
    match tag {
        HOST => {
            info += "host";
        }
        VIEW_TRANSITION if on(log, "view", view_transitions) => {
            info += "ViewTransition";
        }
        VIEW_TRANSITION | FUNCTION | MEMO => {
            if !owned && info.is_empty() {
                info += "function";
            }
        }
        FORWARD_REF if !owned && on(log, "empty", info.is_empty()) => {
            info += "forwardRef";
        }
        _ => {}
    }
    let mut owner = Some(2);
    while let Some(o) = owner {
        info += "/";
        owner = if o > 0 { Some(o - 1) } else { None };
    }
    info
}

/// A guarded arm that isn't last, whose failed guard goes on to nothing.
fn counted(tags: &[u32], log: &RefCell<Vec<String>>) -> u32 {
    let mut total = 0;
    for &tag in tags {
        match tag {
            HOST => total += 1,
            FORWARD_REF if on(log, "even", total % 2 == 0) => total += 20,
            LAZY => total += 300,
            _ => {}
        }
        total += 1;
    }
    total
}

fn main() {
    let log = RefCell::new(Vec::new());
    for tag in [0, 5, 11, 15, 16, 30, 99] {
        for owned in [false, true] {
            for flags in 0..4u32 {
                let flags = [flags & 1 != 0, flags & 2 != 0];
                let info = described(tag, owned, flags, &log);
                println!("{tag} {owned} {flags:?}: {info} {:?}", log.take());
            }
        }
    }
    println!("{} {:?}", counted(&[5, 11, 11, 16, 11, 30, 5, 11], &log), log.take());
}
