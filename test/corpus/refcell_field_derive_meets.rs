//@ run-fail: RefCell already borrowed
// A derived `Clone` borrows a struct's `RefCell` field while what it holds
// runs its own `clone`, which here borrows the field mutably: the field's
// one cell counts the derive's borrow, so the second finds it borrowed, as
// std's does (ADR 0863, amended).
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static HOLDER: RefCell<Option<Rc<Holder>>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct Holder {
    item: RefCell<Item>,
}

enum Item {
    Count(u8),
}

impl Clone for Item {
    fn clone(&self) -> Item {
        if let Some(holder) = HOLDER.with_borrow(|holder| holder.clone()) {
            bump(&holder);
        }
        Item::Count(count_of(self))
    }
}

fn count_of(item: &Item) -> u8 {
    match item {
        Item::Count(n) => *n,
    }
}

fn count(holder: &Holder) -> u8 {
    count_of(&holder.item.borrow())
}

fn bump(holder: &Holder) {
    let n = count(holder);
    holder.item.replace(Item::Count(n + 1));
}

fn main() {
    let holder = Rc::new(Holder {
        item: RefCell::new(Item::Count(1)),
    });
    let first = (*holder).clone();
    println!("{}", count(&first));
    HOLDER.set(Some(Rc::clone(&holder)));
    let second = (*holder).clone();
    println!("{}", count(&second));
}
