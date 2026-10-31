//@ run-fail: RefCell already mutably borrowed
// A trait of classes' method that gives a field lends it where what it
// gives is held elsewhere (rd304): a borrow through that is the field's,
// counted, and the field's own borrow then finds it borrowed, as std's does.
use std::cell::RefCell;

trait Slot {
    fn slot(&self) -> &RefCell<Option<u32>>;
}

#[cfg_attr(rust_js, rust_js::class)]
struct Holder {
    slot: RefCell<Option<u32>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Holder {
    fn new() -> Self {
        Holder {
            slot: RefCell::new(Some(1)),
        }
    }

    fn read(&self) -> Option<u32> {
        *self.slot.borrow()
    }
}

impl Slot for Holder {
    fn slot(&self) -> &RefCell<Option<u32>> {
        &self.slot
    }
}

fn fill(cell: &RefCell<Option<u32>>, holder: &Holder) {
    let mut held = cell.borrow_mut();
    *held = Some(2);
    println!("{:?}", holder.read());
}

fn main() {
    let holder = Holder::new();
    println!("{:?}", holder.read());
    fill(holder.slot(), &holder);
}
