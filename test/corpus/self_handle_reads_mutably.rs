// A closure's handle on itself that holds its cell borrowed mutably when
// it upgrades it: Rust's `borrow` panics, already mutably borrowed. It
// reads its cell by more than the steps of upgrading it, so it's counted
// and checked, as written (ADR 0843, amended).
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::{Rc, Weak};

type Hook = dyn Fn(u32) -> u32;

fn hook() -> Rc<Hook> {
    let this: Rc<RefCell<Option<Weak<Hook>>>> = Rc::new(RefCell::new(None));
    let hook: Rc<Hook> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            if n == 0 {
                return 0;
            }
            let _held = this.borrow_mut();
            let hook = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            hook(n - 1) + 2
        }
    });
    *this.borrow_mut() = Some(Rc::downgrade(&hook));
    hook
}

fn main() {
    let hook = hook();
    println!("{}", hook(0));
    println!("{}", catch_unwind(AssertUnwindSafe(|| hook(1))).is_err());
}
