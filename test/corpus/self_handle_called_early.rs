// A closure's handle on itself called before its cell is set: its upgrade
// gives `None` there, and `unwrap` panics, as Rust runs it. Something runs
// between its `Rc` and its cell's setting, so nothing says the cell holds
// it when it's called: counted, as written (ADR 0843, amended).
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::{Rc, Weak};

type Steps = dyn Fn(u32) -> u32;

fn steps() -> (Rc<Steps>, bool) {
    let this: Rc<RefCell<Option<Weak<Steps>>>> = Rc::new(RefCell::new(None));
    let steps: Rc<Steps> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            if n == 0 {
                return 0;
            }
            let steps = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            steps(n - 1) + 1
        }
    });
    let early = catch_unwind(AssertUnwindSafe(|| steps(1))).is_err();
    *this.borrow_mut() = Some(Rc::downgrade(&steps));
    (steps, early)
}

fn main() {
    let (steps, early) = steps();
    println!("{} {} {}", steps(0), steps(3), early);
}
