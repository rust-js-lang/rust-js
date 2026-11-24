// A `RefCell` field's guard given a name and lent as a `&mut dyn`, `&mut
// *guard`, after another lend of the field as one, `get_mut()`'s: the
// guard's handle on the field is written through by the pair on its
// `value`, so it's the field's place where the pair is, never a `const`
// read of the field, which the pair's setter would assign (#2445). An
// `Option`'s field, a number's, and one with no lend before it. Built as
// a library its borrows are counted, held across a `dyn` call; as a binary
// they're quiet, the guard a handle: test/binaries.test.ts runs it so.
use std::cell::RefCell;

trait Set<T> {
    fn set(&mut self, value: T);
}

impl<T> Set<T> for T {
    fn set(&mut self, value: T) {
        *self = value;
    }
}

struct Slots {
    slot: RefCell<Option<u32>>,
    count: RefCell<u32>,
}

fn lent_again() {
    let mut slots = Slots {
        slot: RefCell::new(None),
        count: RefCell::new(0),
    };
    let first: &mut dyn Set<Option<u32>> = slots.slot.get_mut();
    first.set(Some(1));
    {
        let mut guard = slots.slot.borrow_mut();
        let then: &mut dyn Set<Option<u32>> = &mut *guard;
        then.set(Some(2));
    }
    println!("{:?}", slots.slot.borrow());
    let first: &mut dyn Set<u32> = slots.count.get_mut();
    first.set(3);
    {
        let mut guard = slots.count.borrow_mut();
        let then: &mut dyn Set<u32> = &mut *guard;
        then.set(4);
    }
    println!("{}", slots.count.borrow());
}

fn lent_once() {
    let slots = Slots {
        slot: RefCell::new(None),
        count: RefCell::new(0),
    };
    {
        let mut guard = slots.slot.borrow_mut();
        let then: &mut dyn Set<Option<u32>> = &mut *guard;
        then.set(Some(5));
    }
    println!("{:?} {}", slots.slot.borrow(), slots.count.borrow());
}

fn main() {
    lent_again();
    lent_once();
}
