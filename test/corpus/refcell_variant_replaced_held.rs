//@ run-fail: RefCell already borrowed
// An enum in a `RefCell` whose variant's own `RefCell` field is borrowed:
// replacing the variant while that borrow is alive finds the enum's cell
// borrowed, as std's does (ADR 0863, amended, #503).
use std::cell::RefCell;
use std::rc::Rc;

pub enum Kind {
    Effect { cleanup: RefCell<Option<u32>> },
    Scope,
}

fn replace(kind: &RefCell<Kind>) {
    *kind.borrow_mut() = Kind::Scope;
}

fn main() {
    let kind = Rc::new(RefCell::new(Kind::Effect {
        cleanup: RefCell::new(Some(3)),
    }));
    replace(&kind);
    *kind.borrow_mut() = Kind::Effect {
        cleanup: RefCell::new(Some(4)),
    };
    let outer = kind.borrow();
    if let Kind::Effect { cleanup } = &*outer {
        let inner = cleanup.borrow();
        println!("{:?}", *inner);
        replace(&kind);
        println!("{:?}", *inner);
    }
}
