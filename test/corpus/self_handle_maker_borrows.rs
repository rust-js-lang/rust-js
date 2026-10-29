// A closure's handle on itself whose maker holds a borrow of the cell
// when it sets it: Rust's `borrow_mut` panics, already borrowed. Its cell
// is read by more than the steps of making and setting it, so it's
// counted and checked, as written (ADR 0843, amended).
use std::cell::RefCell;
use std::panic::catch_unwind;
use std::rc::{Rc, Weak};

type Hook = dyn Fn(u32) -> u32;

fn hook() -> Rc<Hook> {
    let this: Rc<RefCell<Option<Weak<Hook>>>> = Rc::new(RefCell::new(None));
    let _held = this.borrow();
    let hook: Rc<Hook> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            if n == 0 {
                return 0;
            }
            let hook = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            hook(n - 1) + 2
        }
    });
    *this.borrow_mut() = Some(Rc::downgrade(&hook));
    hook
}

fn main() {
    println!("{}", catch_unwind(hook).is_err());
}
