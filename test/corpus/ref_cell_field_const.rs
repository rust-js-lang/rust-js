// A `const` of a struct whose `RefCell` field is the value it holds (ADR
// 0863, amended): its `None` was the cell's `{ value }`, which a read of the
// field took for `Some`.
use std::cell::RefCell;

struct Slot {
    items: RefCell<Option<u8>>,
    count: u32,
}

const EMPTY: Slot = Slot {
    items: RefCell::new(None),
    count: 0,
};

const FULL: Slot = Slot {
    items: RefCell::new(Some(4)),
    count: 1,
};

fn filled(slot: &Slot) -> bool {
    slot.items.borrow().is_some()
}

fn main() {
    println!("{} {}", EMPTY.items.borrow().is_some(), EMPTY.count);
    println!("{} {}", filled(&EMPTY), filled(&FULL));
    let slot = FULL;
    *slot.items.borrow_mut() = None;
    println!("{:?} {:?} {}", slot.items.borrow(), FULL.items.borrow(), FULL.count);
}
