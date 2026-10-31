// `as_deref()` of an `Option` of a `RefCell`'s guard, which clippy's
// `option_as_ref_deref` asks for: what the guard derefs to, its cell's
// value, or `None`.

use std::cell::{Ref, RefCell};

struct Transition {
    types: RefCell<Vec<String>>,
    count: RefCell<u32>,
    started: bool,
}

// A guard of a number's cell, which derefs to its value.
fn count_of(transition: &Transition) -> Option<u32> {
    let count = transition.started.then(|| transition.count.borrow());
    count.as_deref().copied()
}

fn types_of(transition: &Transition) -> Option<Ref<'_, Vec<String>>> {
    transition.started.then(|| transition.types.borrow())
}

fn shown(transition: &Transition) -> String {
    let types = types_of(transition);
    match types.as_deref() {
        Some(types) => types.join(","),
        None => "none".to_string(),
    }
}

fn main() {
    let started = Transition {
        types: RefCell::new(vec!["a".to_string(), "b".to_string()]),
        count: RefCell::new(3),
        started: true,
    };
    let idle = Transition {
        types: RefCell::new(Vec::new()),
        count: RefCell::new(0),
        started: false,
    };
    println!("{}", shown(&started));
    started.types.borrow_mut().push("c".to_string());
    println!("{}", shown(&started));
    println!("{}", shown(&idle));
    println!("{}", types_of(&started).as_deref().map_or(0, Vec::len));
    *started.count.borrow_mut() += 1;
    println!("{:?} {:?}", count_of(&started), count_of(&idle));
}
