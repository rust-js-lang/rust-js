// A `match` of strings that's a `switch`, an arm of which tests in its
// body, `"focusin" => { if .. { .. } }`, and moves a value only there,
// whose drop MIR then decides at the end: the arm's `if` is its case's,
// not part of its test, so the `match` is still a `switch`, as React DOM's
// SelectEventPlugin writes it, `case 'focusin': if (..) { activeElementInst
// = targetInst; .. } break;`, with each arm's comment above its `case`
// (rd619).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    static SKIP: Cell<bool> = const { Cell::new(false) };
    static DOWN: Cell<bool> = const { Cell::new(false) };
    static INST: RefCell<Option<Rc<String>>> = const { RefCell::new(None) };
}

fn construct(log: &RefCell<Vec<String>>, n: i32) {
    let inst = INST.with_borrow(|inst| inst.as_deref().cloned());
    log.borrow_mut().push(format!("construct {n} {} {inst:?}", DOWN.get()));
}

fn extract(name: &str, n: i32, target: Option<Rc<String>>, log: &RefCell<Vec<String>>) {
    let node = match &target {
        Some(target) => target.len(),
        None => 0,
    };
    match name {
        // Track the input node that has focus.
        "focusin" => {
            if node > 2 || log.borrow().is_empty() {
                INST.set(target);
            }
        }
        "focusout" => {
            INST.set(None);
        }
        // Don't fire the event while the user is dragging.
        "mousedown" => {
            DOWN.set(true);
        }
        "contextmenu" | "mouseup" | "dragend" => {
            DOWN.set(false);
            construct(log, n);
        }
        // Chrome and IE fire non-standard event when selection is changed.
        //
        // This is also our approach for IE handling.
        "selectionchange" if SKIP.get() => {}
        "selectionchange" | "keydown" | "keyup" => {
            construct(log, n);
        }
        _ => {}
    }
}

// No guard: the arm's `if` is still its case's.
fn focus(name: &str, n: i32, target: Option<Rc<String>>, log: &RefCell<Vec<String>>) {
    let node = match &target {
        Some(target) => target.len(),
        None => 0,
    };
    match name {
        "focusin" => {
            if node > 2 || log.borrow().is_empty() {
                INST.set(target);
            }
        }
        "focusout" => {
            INST.set(None);
        }
        "contextmenu" | "mouseup" | "dragend" => {
            DOWN.set(false);
            construct(log, n);
        }
        _ => {}
    }
}

fn main() {
    let log = RefCell::new(Vec::new());
    let input = Rc::new("input".to_string());
    for name in ["focusin", "mousedown", "selectionchange", "mouseup", "keyup", "other"] {
        extract(name, 3, Some(Rc::clone(&input)), &log);
    }
    extract("focusin", 5, Some(Rc::new("a".to_string())), &log);
    extract("keydown", 5, None, &log);
    extract("focusout", 6, None, &log);
    SKIP.set(true);
    for name in ["selectionchange", "keydown", "mousedown", "selectionchange"] {
        extract(name, 4, None, &log);
    }
    focus("focusin", 7, Some(Rc::new("bc".to_string())), &log);
    extract("keyup", 7, None, &log);
    focus("mouseup", 8, None, &log);
    focus("focusout", 9, None, &log);
    extract("keyup", 8, None, &log);
    println!("{:?}", log.borrow());
}
