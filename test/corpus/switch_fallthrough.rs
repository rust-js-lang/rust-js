// A `match` of constants that repeats a constant under guards, each arm's
// constants among the next one's: React's `getComponentNameFromFiber`, a
// `switch` whose guarded cases fall through to the next (ADR 0899). Each
// guard runs where Rust runs it, after its constant matched, and only
// until one holds.
use std::cell::RefCell;

const ACTIVITY: u32 = 1;
const VIEW_TRANSITION: u32 = 2;
const INCOMPLETE_CLASS: u32 = 3;
const INCOMPLETE_FUNCTION: u32 = 4;
const CLASS: u32 = 5;
const FUNCTION: u32 = 6;
const LEGACY_HIDDEN: u32 = 7;
const THROW: u32 = 8;

/// A guard that says it ran.
fn on(log: &RefCell<Vec<String>>, name: &str, value: bool) -> bool {
    log.borrow_mut().push(name.to_string());
    value
}

fn name(tag: u32, ty: &str, flags: [bool; 3], log: &RefCell<Vec<String>>) -> Option<String> {
    let [view_transitions, legacy_off, legacy_hidden] = flags;
    match tag {
        ACTIVITY => return Some("Activity".to_string()),
        VIEW_TRANSITION if on(log, "view", view_transitions) => {
            return Some("ViewTransition".to_string());
        }
        // The display name for these tags come from the user-provided type:
        VIEW_TRANSITION | INCOMPLETE_CLASS | INCOMPLETE_FUNCTION if on(log, "legacy", legacy_off) => {}
        VIEW_TRANSITION | INCOMPLETE_CLASS | INCOMPLETE_FUNCTION | CLASS | FUNCTION => {
            if ty.len() > 3 {
                return Some(ty.to_string());
            }
            if ty.is_empty() {
                return Some("empty".to_string());
            }
        }
        LEGACY_HIDDEN => {
            if legacy_hidden {
                return Some("LegacyHidden".to_string());
            }
        }
        THROW => {
            if ty.len() == 1 {
                return Some(ty.to_uppercase());
            }
            return Some("Throw".to_string());
        }
        _ => {}
    }
    log.borrow_mut().push("none".to_string());
    None
}

/// A guarded arm whose body goes on past the `match`, in a loop, whose
/// `break` and `continue` are the loop's.
fn counted(tags: &[u32], log: &RefCell<Vec<String>>) -> u32 {
    let mut total = 0;
    for &tag in tags {
        match tag {
            1 => total += 1,
            2 if on(log, "two", total % 2 == 0) => total += 20,
            2 | 3 if on(log, "three", total > 40) => break,
            2 | 3 | 4 => total += 300,
            5 if on(log, "five", total > 100) => continue,
            5 | 6 => total += 4000,
            _ => {}
        }
        total += 1;
    }
    total
}

fn main() {
    let log = RefCell::new(Vec::new());
    for tag in 0..10 {
        for ty in ["", "a", "div", "Button"] {
            for flags in 0..8u32 {
                let flags = [flags & 1 != 0, flags & 2 != 0, flags & 4 != 0];
                let named = name(tag, ty, flags, &log);
                println!("{tag} {ty:?} {flags:?}: {named:?} {:?}", log.take());
            }
        }
    }
    for tags in [&[1, 2, 2, 3, 4][..], &[2, 3, 5, 5, 6], &[4, 4, 2, 3, 3, 1], &[5, 1, 2, 9, 5]] {
        let total = counted(tags, &log);
        println!("{tags:?}: {total} {:?}", log.take());
    }
}
