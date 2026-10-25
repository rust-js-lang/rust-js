// A `let` that ends a `RefCell`'s borrow, which an `if let` then takes
// apart, React's `startTransition`'s `let prev_types =
// prev.types.borrow().clone(); if DEV && let Some(prev_types) = prev_types
// && ..`: where only that test reads it, and the test does nothing, the
// place is read there, `prev.types != null && prev.types !== currentTypes`
// (ADR 0866). Not where the body reads it too, nor where the test runs
// code that may change the cell first, nor a `let` no `if let` takes apart.
use std::cell::RefCell;
use std::rc::Rc;

const DEV: bool = true;

struct Transition {
    types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
}

fn transition(types: &[&str]) -> Transition {
    let types = (!types.is_empty()).then(|| Rc::new(RefCell::new(types.iter().map(|t| t.to_string()).collect())));
    Transition {
        types: RefCell::new(types),
    }
}

fn finish(prev: Option<&Transition>, current: &Transition) {
    let current_types = current.types.borrow().clone();
    if let Some(prev) = prev
        && let Some(current_types) = current_types
    {
        let prev_types = prev.types.borrow().clone();
        if DEV
            && let Some(prev_types) = prev_types
            && !Rc::ptr_eq(&prev_types, &current_types)
        {
            println!("overriding {:?}", prev.types.borrow());
        }
        prev.types.replace(Some(current_types));
    }
}

/// What the test runs first may change the cell: read before it, as Rust
/// reads it.
fn emptied(t: &Transition) -> bool {
    t.types.replace(None).is_some()
}

fn read_first(t: &Transition) -> usize {
    let types = t.types.borrow().clone();
    if emptied(t)
        && let Some(types) = types
        && types.borrow().len() > 1
    {
        return 2;
    }
    0
}

/// One the body reads too: one read, before the test.
fn counted(t: &Transition) -> usize {
    let types = t.types.borrow().clone();
    if let Some(types) = types {
        types.borrow_mut().push("counted".to_string());
        return types.borrow().len();
    }
    0
}

/// One no `if let` takes apart, tested: its name kept.
fn unset(t: &Transition) -> bool {
    let types = t.types.borrow().clone();
    if DEV && types.is_none() {
        return true;
    }
    false
}

fn main() {
    let outer = transition(&["a"]);
    let inner = transition(&["b"]);
    finish(Some(&outer), &inner);
    finish(Some(&outer), &inner);
    finish(None, &inner);
    finish(Some(&transition(&[])), &inner);
    println!("outer {:?}", outer.types.borrow().as_ref().map(|t| t.borrow().clone()));
    println!(
        "read first {} {}",
        read_first(&transition(&["x", "y"])),
        read_first(&transition(&[]))
    );
    println!("counted {} {}", counted(&outer), counted(&transition(&[])));
    println!("unset {} {}", unset(&outer), unset(&transition(&[])));
}
