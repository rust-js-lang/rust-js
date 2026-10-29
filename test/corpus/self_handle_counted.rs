// A closure's handle on itself that's counted, as the program wrote it
// (ADR 0843, amended): one whose count the program reads, and one whose
// cell it reads apart, before it's set, which upgrades to `None` there.
// Every `dyn Fn`'s `Rc` here is counted, as one counted may be any other
// (ADR 0852), so these are apart from `self_handle`'s.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Counted = dyn Fn(u64) -> u64;

// The same, but the program reads its count: counted, as written.
fn counted(step: u64) -> (Rc<Counted>, usize) {
    let this: Rc<RefCell<Option<Weak<Counted>>>> = Rc::new(RefCell::new(None));
    let countdown: Rc<Counted> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            let countdown = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            if n < step { Rc::strong_count(&countdown) as u64 } else { countdown(n - step) }
        }
    });
    *this.borrow_mut() = Some(Rc::downgrade(&countdown));
    let strong = Rc::strong_count(&countdown);
    (countdown, strong)
}

type Probe = dyn Fn() -> bool;

// Its cell read apart, before it's set: upgraded as Rust does, `None`.
fn read_apart() -> (Rc<Probe>, bool) {
    let this: Rc<RefCell<Option<Weak<Probe>>>> = Rc::new(RefCell::new(None));
    let probe: Rc<Probe> = Rc::new({
        let this = Rc::clone(&this);
        move || this.borrow().as_ref().and_then(Weak::upgrade).is_some()
    });
    let before = this.borrow().is_some();
    *this.borrow_mut() = Some(Rc::downgrade(&probe));
    (probe, before)
}

fn main() {
    let (counted, strong) = counted(2);
    println!("{} {}", counted(5), strong);

    let (probe, before) = read_apart();
    println!("{} {}", probe(), before);
}
