//@ run-fail: RefCell already borrowed
// A cell is each value it flows to (ADR 0863, amended): put in a `Vec` and
// a `HashMap` by its `Rc`, found again by `get`, `cloned` and `map` in a
// closure called while a borrow of it from the map is held, it's the
// held one's, so both are checked and the second panics, as std's does.
// Another of its type, which no held borrow can be, is never checked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct Registry {
    cells: Vec<Rc<RefCell<Vec<i32>>>>,
}

fn find(registry: &Registry, at: usize) -> Option<Rc<RefCell<Vec<i32>>>> {
    registry.cells.get(at).cloned()
}

fn main() {
    let made = Rc::new(RefCell::new(vec![1]));
    let mut registry = Registry { cells: Vec::new() };
    registry.cells.push(Rc::clone(&made));
    let mut by_name: HashMap<&str, Rc<RefCell<Vec<i32>>>> = HashMap::new();
    by_name.insert("a", made);
    let other = Rc::new(RefCell::new(vec![7]));
    other.borrow_mut().push(8);
    println!("{}", other.borrow().len());
    let touch = |registry: &Registry| find(registry, 0).map(|cell| cell.borrow_mut().push(2));
    let held = by_name["a"].borrow();
    println!("{}", held.len());
    touch(&registry);
    println!("{}", held.len());
}
