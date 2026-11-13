// A `clone_from` drops the value it's lent `&mut`, writing a clone over
// it: here std's own, of an `Rc`, in a helper given only a shared
// reference to a cell of it, while a guard of another cell, which the
// old value's destructor borrows, is held. Native's destructor finds the
// log borrowed (#1661: `std_call_drops`' value lent `&mut` that std can
// make another of, which the `RefMut` it's reached through also counts).
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

struct Entry {
    name: &'static str,
}

impl Drop for Entry {
    fn drop(&mut self) {
        let free = LOG.with(|log| log.try_borrow_mut().is_ok());
        println!("{} dropped, log free: {free}", self.name);
    }
}

fn reset(slot: &RefCell<Rc<Entry>>, new: &Rc<Entry>) {
    slot.borrow_mut().clone_from(new);
}

fn main() {
    let slot = RefCell::new(Rc::new(Entry { name: "old" }));
    let new = Rc::new(Entry { name: "new" });
    LOG.with(|log| {
        let mut held = log.borrow_mut();
        held.push("cloning");
        reset(&slot, &new);
        held.push(slot.borrow().name);
    });
    LOG.with_borrow(|log| println!("{log:?}"));
}
