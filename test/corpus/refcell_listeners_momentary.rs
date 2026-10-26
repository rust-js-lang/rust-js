// A shared list of `&dyn Fn()` listeners, each borrow of it a moment's:
// none is held while a listener runs, so none can meet another, and the
// list is the array, unchecked (ADRs 0580, 0863). A `RefCell<Vec<&dyn
// Fn()>>` has no `PartialEq` nor `Debug`, as a `dyn Fn()` hasn't, so no
// impl of its own that would run one borrows it. Read through its `Rc`,
// `i < listeners.borrow().len()` puts `i` below the length, so the item's
// read and `i += 1` are plain (ADR 0881). A listener may add another,
// which the loop then calls too.

use std::cell::RefCell;
use std::rc::Rc;

type Listener = &'static dyn Fn();

struct Thenable {
    then: Box<dyn Fn(Listener)>,
    resolve: Box<dyn Fn()>,
}

fn thenable() -> Thenable {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    Thenable {
        then: {
            let listeners = Rc::clone(&listeners);
            Box::new(move |listener| listeners.borrow_mut().push(listener))
        },
        resolve: Box::new(move || {
            let mut i = 0;
            while i < listeners.borrow().len() {
                let listener = listeners.borrow()[i];
                listener();
                i += 1;
            }
        }),
    }
}

fn first() {
    println!("first");
}

fn second() {
    println!("second");
}

fn main() {
    let thenable: &'static Thenable = Box::leak(Box::new(thenable()));
    (thenable.then)(&first);
    (thenable.then)(Box::leak(Box::new(move || {
        println!("adding");
        (thenable.then)(&second);
    })));
    (thenable.resolve)();
    println!("again");
    (thenable.resolve)();
}
