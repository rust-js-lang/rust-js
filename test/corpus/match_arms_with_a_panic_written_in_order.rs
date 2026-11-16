// A `match` whose arm that panics is written between the others, as React's
// Fizz server matches `ResumeSlots`: `None => .., Root(_) => panic!(..),
// Indices(slots) => slots`, each of two arms of `let slots = match key_path`.
// Its chain reads in the arms' order, `} else if (typeof slots$1 ===
// "number") { throw .. } else { slots = slots$1; }`, whether the guard of
// the arm's `RefCell` is counted, a `try` around it, or checked alone
// (#1972, #1815). A `let … else`'s panic in an arm is its guard, first,
// as written: `if (parentNext == null) { throw .. }`.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
enum Slots {
    #[cfg_attr(rust_js, rust_js::null)]
    None,
    Root(f64),
    Indices(&'static str),
}

struct Node {
    slots: Slots,
}

struct Holes {
    root_slots: Cell<Slots>,
    working_map: RefCell<HashMap<u32, Rc<RefCell<Node>>>>,
    next: Cell<u32>,
}

fn new_dict(holes: &Holes) -> &'static str {
    holes.next.set(holes.next.get() + 1);
    ["a", "b", "c", "d"][holes.next.get() as usize % 4]
}

fn track(holes: &Holes, key_path: Option<u32>, index: u32) {
    let slots = match key_path {
        None => {
            let slots = holes.root_slots.get();
            match slots {
                Slots::None => {
                    let slots = new_dict(holes);
                    holes.root_slots.set(Slots::Indices(slots));
                    slots
                }
                Slots::Root(_) => {
                    panic!("both at the root and a slot below");
                }
                Slots::Indices(slots) => slots,
            }
        }
        Some(key_path) => {
            let node = holes.working_map.borrow().get(&key_path).cloned();
            match node {
                None => {
                    let slots = new_dict(holes);
                    let node = Rc::new(RefCell::new(Node {
                        slots: Slots::Indices(slots),
                    }));
                    holes.working_map.borrow_mut().insert(key_path, node);
                    slots
                }
                Some(node) => {
                    let mut node = node.borrow_mut();
                    let slots = node.slots;
                    match slots {
                        Slots::None => {
                            let slots = new_dict(holes);
                            node.slots = Slots::Indices(slots);
                            slots
                        }
                        Slots::Root(_) => {
                            panic!("both at the root and a slot below");
                        }
                        Slots::Indices(slots) => slots,
                    }
                }
            }
        }
    };
    println!("{slots} {index}");
}

fn common(prev: Option<&u32>, next: Option<&u32>, seen: &mut Vec<u32>) {
    match prev {
        None => {
            if next.is_some() {
                panic!("the stacks must reach the root at the same time");
            }
        }
        Some(prev) => {
            let Some(next) = next else {
                panic!("the stacks must reach the root at the same time");
            };
            seen.push(prev + next);
        }
    }
    seen.push(0);
}

fn main() {
    let mut seen = Vec::new();
    common(None, None, &mut seen);
    common(Some(&1), Some(&2), &mut seen);
    println!("{seen:?}");
    let holes = Holes {
        root_slots: Cell::new(Slots::None),
        working_map: RefCell::new(HashMap::new()),
        next: Cell::new(0),
    };
    track(&holes, None, 1);
    track(&holes, None, 2);
    track(&holes, Some(3), 3);
    track(&holes, Some(3), 4);
    let n = holes.working_map.borrow().get(&3).cloned().unwrap();
    let call = |f: &dyn Fn()| {
        let _held = n.borrow();
        f();
    };
    call(&|| println!("{}", n.try_borrow_mut().is_err()));
}
