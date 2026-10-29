// A `match` of strings whose guarded arm of no statements falls to the
// next, which names its string and more: React DOM's SelectEventPlugin,
// `"selectionchange" if SKIP.with(|skip| *skip) => {}`, upstream's `case
// 'selectionchange': if (skipSelectionChangeEvent) { break; } // falls
// through`, its arms' comments before their bodies, `case 'mousedown': //
// Don't fire ..`, and no `default:` of the last `_ => {}` (rd119, ADR 0905
// amended).
use std::cell::{Cell, RefCell};

thread_local! {
    static SKIP: Cell<bool> = const { Cell::new(false) };
    static DOWN: Cell<bool> = const { Cell::new(false) };
}

fn construct(log: &RefCell<Vec<String>>, n: i32) {
    log.borrow_mut().push(format!("construct {n} {}", DOWN.get()));
}

fn extract(name: &str, n: i32, log: &RefCell<Vec<String>>) {
    match name {
        // Track the input node that has focus.
        "focusin" => {
            construct(log, 1);
        }
        "focusout" => {
            construct(log, 2);
        }
        // Don't fire the event while the user is dragging. This matches the
        // semantics of the native select event.
        "mousedown" => {
            DOWN.set(true);
        }
        "contextmenu" | "mouseup" | "dragend" => {
            DOWN.set(false);
            construct(log, n);
        }
        // Chrome and IE fire non-standard event when selection is changed (and
        // sometimes when it hasn't). IE's event fires out of order with respect
        // to key and input events on deletion, so we discard it.
        "selectionchange" if SKIP.get() => {}
        "selectionchange" | "keydown" | "keyup" => {
            construct(log, n);
        }
        _ => {}
    }
}

fn main() {
    let log = RefCell::new(Vec::new());
    for name in ["focusin", "mousedown", "selectionchange", "mouseup", "keyup", "other"] {
        extract(name, 3, &log);
    }
    SKIP.set(true);
    for name in ["selectionchange", "keydown", "mousedown", "selectionchange"] {
        extract(name, 4, &log);
    }
    println!("{:?}", log.borrow());
}
