// A value taken from an `Option`, or made and kept where it's `None`, then
// used, `let handles = match handles { Some(h) => h, None => { make; keep;
// h } };`: one variable, set where it's none, `let handles = ..; if
// (handles == null) { handles = ..; } handles.add(..)`, as upstream's
// addEventHandleToTarget writes it (React port gap rd65).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

type List = Rc<RefCell<Vec<u32>>>;

fn add_to(slot: &RefCell<Option<List>>, value: u32, made: &Cell<u32>) {
    let list = slot.borrow().clone();
    let list = match list {
        Some(list) => list,
        None => {
            made.set(made.get() + 1);
            let list = Rc::new(RefCell::new(Vec::new()));
            *slot.borrow_mut() = Some(list.clone());
            list
        }
    };
    list.borrow_mut().push(value);
}

fn main() {
    let slot = RefCell::new(None);
    let made = Cell::new(0);
    add_to(&slot, 1, &made);
    add_to(&slot, 2, &made);
    let list = slot.borrow().clone().expect("made");
    println!("{:?} {}", list.borrow(), made.get());
}
