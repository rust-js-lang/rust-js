// A let-else's scrutinee's guard lives while its initializer runs, and is
// dropped before its `else` does (ADR 0863, amended): React's walk to the
// last update, `let Some(next) = update.next.borrow().clone() else {
// break }`, is a borrow for a moment, so the field stays its value.
use std::cell::RefCell;
use std::rc::Rc;

struct Update {
    lane: u32,
    next: RefCell<Option<Rc<Update>>>,
}

fn last(first: &Rc<Update>) -> Rc<Update> {
    let mut update = Rc::clone(first);
    loop {
        let Some(next) = update.next.borrow().clone() else {
            break;
        };
        update = next;
    }
    update
}

fn link(update: &Update, next: &Rc<Update>) {
    update.next.replace(Some(Rc::clone(next)));
}

fn main() {
    let a = Rc::new(Update { lane: 1, next: RefCell::new(None) });
    let b = Rc::new(Update { lane: 2, next: RefCell::new(None) });
    link(&a, &b);
    println!("{} {}", last(&a).lane, last(&b).lane);

    // Borrowed while the initializer runs, and free in the `else`.
    let cell: RefCell<Option<u32>> = RefCell::new(Some(4));
    let Some((x, free)) = cell.borrow().map(|v| (v, cell.try_borrow_mut().is_ok())) else {
        return;
    };
    println!("{x} {free}");
    let empty: RefCell<Option<u32>> = RefCell::new(None);
    let Some(y) = *empty.borrow() else {
        *empty.borrow_mut() = Some(5);
        println!("{:?} {}", empty.borrow(), empty.try_borrow_mut().is_ok());
        return;
    };
    println!("{y}");
}
