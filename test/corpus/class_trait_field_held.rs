//@ run-fail: RefCell already mutably borrowed
// A default's call of a trait of classes' method, of a `Self` not known,
// runs what each class's does (rd304): a borrow held across it is counted
// where one's borrows its cell, and that borrow finds it borrowed, as
// std's does.
use std::cell::RefCell;

trait Guard {
    fn flag(&self) -> &RefCell<Option<i8>>;
    fn probe(&self) -> Option<i8>;

    fn guarded(&self) -> Option<i8> {
        let _held = self.flag().borrow_mut();
        self.probe()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Gate {
    flag: RefCell<Option<i8>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Gate {
    fn new() -> Self {
        Gate {
            flag: RefCell::new(Some(1)),
        }
    }
}

impl Guard for Gate {
    fn flag(&self) -> &RefCell<Option<i8>> {
        &self.flag
    }

    fn probe(&self) -> Option<i8> {
        *self.flag.borrow()
    }
}

fn main() {
    let gate = Gate::new();
    println!("{:?}", gate.probe());
    println!("{:?}", gate.guarded());
}
