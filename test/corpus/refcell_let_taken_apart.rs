// A `let` that ends a `RefCell`'s borrow, which an `if let` then takes
// apart, React's `startTransition`'s `let prev_types =
// prev.types.borrow().clone(); if DEV && let Some(prev_types) = prev_types
// && ..`: a `let` the program wrote, which stays, `const prevTypes =
// prev.types;` (rd226, the user's decision). A binding a pattern alone
// makes of a field by its own name, read by the test and the `then`'s first
// statement alone, is the field read in place, React's
// getViewTransitionName's `if (instance.autoName != null) { return
// instance.autoName; }` (ADR 0866, amended); not where the body reads it
// again, nor where the test runs code.
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

struct Named {
    auto_name: RefCell<Option<String>>,
}

/// A pattern's binding of a field by its own name, read by the test and the
/// `then`'s first statement alone: the field read there.
fn own(named: &Named) -> String {
    if let Some(auto_name) = &*named.auto_name.borrow() {
        return auto_name.clone();
    }
    "none".to_string()
}

/// One the body reads twice: held, before the test.
fn own_twice(named: &Named, seen: &mut Vec<String>) -> usize {
    if let Some(auto_name) = &*named.auto_name.borrow() {
        seen.push(auto_name.clone());
        return auto_name.len();
    }
    0
}

/// One whose test runs code: held, before the test.
fn own_checked(named: &Named) -> String {
    if let Some(auto_name) = &*named.auto_name.borrow()
        && checked(auto_name)
    {
        return auto_name.clone();
    }
    "unchecked".to_string()
}

fn checked(name: &str) -> bool {
    println!("checked {name}");
    !name.is_empty()
}

/// A `let` the program wrote, named as the field, its `if let`'s body
/// reading it once: its name kept.
fn first(t: &Transition) -> Option<Rc<RefCell<Vec<String>>>> {
    let types = t.types.borrow().clone();
    if let Some(types) = types {
        return Some(types);
    }
    None
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
    let named = Named {
        auto_name: RefCell::new(Some("a".to_string())),
    };
    let unnamed = Named {
        auto_name: RefCell::new(None),
    };
    println!("own {} {}", own(&named), own(&unnamed));
    let mut seen = Vec::new();
    println!("own twice {} {}", own_twice(&named, &mut seen), own_twice(&unnamed, &mut seen));
    println!("seen {seen:?}");
    println!("own checked {} {}", own_checked(&named), own_checked(&unnamed));
    println!("first {:?} {:?}", first(&outer).map(|t| t.borrow().len()), first(&transition(&[])).is_none());
}
