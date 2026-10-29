// React's `getComponentNameFromFiber`, with a guarded arm of a constant no
// arm names again, whose guard fails past the `match`: a `switch` still,
// its guarded cases falling through to the next, and the last guard's case
// going on to what follows the `match`, not to another arm's statements
// (ADR 0905).
use std::cell::RefCell;

const ACTIVITY: u32 = 1;
const VIEW_TRANSITION: u32 = 2;
const INCOMPLETE_CLASS: u32 = 3;
const CLASS: u32 = 5;
const LEGACY_HIDDEN: u32 = 7;

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
        VIEW_TRANSITION | INCOMPLETE_CLASS if on(log, "legacy", legacy_off) => {}
        VIEW_TRANSITION | INCOMPLETE_CLASS | CLASS => {
            if ty.starts_with('<') {
                return Some(ty.to_string());
            }
        }
        LEGACY_HIDDEN if on(log, "hidden", legacy_hidden) => {
            if ty.is_empty() {
                return Some("LegacyHidden".to_string());
            }
        }
        _ => {}
    }
    None
}

fn main() {
    let log = RefCell::new(Vec::new());
    for tag in 0..9 {
        for ty in ["", "a", "<b>"] {
            for flags in 0..8u32 {
                let flags = [flags & 1 != 0, flags & 2 != 0, flags & 4 != 0];
                let named = name(tag, ty, flags, &log);
                println!("{tag} {ty:?} {flags:?}: {named:?} {:?}", log.take());
            }
        }
    }
}
