// A guard a `let` holds while an `Rc` it guards is cloned out into the
// closure's result, `Some(Rc::clone(&history.entries[i]))`: an `Rc`'s
// count asks no cell, and `State` is an `i32`, whose drop runs nothing,
// so nothing can ask whether the cell is borrowed, and no guard of it is
// counted (#1815).
use std::cell::RefCell;
use std::rc::Rc;

struct History<State> {
    entries: Vec<Rc<State>>,
    index: usize,
}

fn stepper<State>(history: Rc<RefCell<History<State>>>) -> impl Fn() -> Option<Rc<State>> {
    move || {
        let mut history = history.borrow_mut();
        if history.index == 0 {
            return None;
        }
        history.index -= 1;
        Some(Rc::clone(&history.entries[history.index]))
    }
}

fn main() {
    let history = Rc::new(RefCell::new(History {
        entries: vec![Rc::new(1), Rc::new(2), Rc::new(3)],
        index: 2,
    }));
    let back = stepper(Rc::clone(&history));
    println!("{:?} {:?} {:?}", back(), back(), back());
    println!(
        "{} {}",
        history.borrow().index,
        Rc::strong_count(&history.borrow().entries[0])
    );
}
