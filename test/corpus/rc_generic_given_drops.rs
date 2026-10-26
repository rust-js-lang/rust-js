// An `Rc` of a value of a type parameter's is counted where a caller gives
// that parameter a value whose drop runs a destructor (ADR 0914, amended):
// here `keep` is given a `Loud`, so the last clone of its `Rc<Slot<Loud>>`
// drops it, and not the first; and its `Slot<u32>`, which may be one, is
// counted with it. A `T: Clone` given the counted `Rc` clones it as one
// more of its count, given a number, as the number.
use std::cell::RefCell;
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Slot<T> {
    value: RefCell<T>,
}

fn keep<T>(value: T) -> Rc<Slot<T>> {
    Rc::new(Slot {
        value: RefCell::new(value),
    })
}

fn shared<T>(slot: &Rc<Slot<T>>) -> Rc<Slot<T>> {
    Rc::clone(slot)
}

fn read<T: Clone>(slot: &Slot<T>) -> T {
    slot.value.borrow().clone()
}

// Given a counted `Rc`, whose clone is one more of its count.
fn cloned<T: Clone>(value: &T) -> T {
    value.clone()
}

struct Held<T> {
    value: T,
}

impl<T> Held<T> {
    fn get(&self) -> &T {
        &self.value
    }
}

// An `Rc` only generic code has, of what its caller gives it: counted where
// one gives it a `Loud`, its last clone's drop the one that drops it.
fn churn<T>(value: T) {
    let first = Rc::new(Held { value });
    let second = Rc::clone(&first);
    drop(first);
    println!("one held");
    let _value = second.get();
    drop(second);
    println!("none held");
}

fn main() {
    let numbers = keep(1u32);
    let more = shared(&numbers);
    *more.value.borrow_mut() += 1;
    println!("{} {}", read(&numbers), read(&more));
    let loud = keep(Loud(7));
    let other = shared(&loud);
    println!("{}", loud.value.borrow().0);
    let again = cloned(&other);
    drop(loud);
    println!("two left");
    drop(other);
    println!("one left");
    drop(again);
    println!("none left");
    churn(Loud(9));
    churn(3u8);
}
