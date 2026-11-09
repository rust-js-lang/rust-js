// What generic code counts is what its callers give it (#1009): a counted
// `Rc<Slot<T>>` is each `Rc<Slot<_>>` its callers give, `Slot<u32>` here,
// and a trait method's counted `Rc<T>` each `T` its namings give, `Item`,
// though the trait is a dictionary's (#1036). No other `Rc` is counted, so
// an untagged enum's variants, which JS tells apart by their `Rc`s' values
// (ADR 0214), are told apart: `Rc<Slot<String>>` and `Rc<Label>`. Nor is
// one whose drop asks whether its own `Rc` is, `Label`'s `next`, beside a
// bare `Rc<T>` whose callers give `Loud` and `u8` (seed 57 of the neighbour
// shuffle).
use std::cell::RefCell;
use std::rc::Rc;

struct Slot<T> {
    value: T,
}

fn count_slot<T>(slot: &Rc<Slot<T>>) -> usize {
    Rc::strong_count(slot)
}

trait Counts {
    fn count_of<T>(&self, rc: &Rc<T>) -> usize;
}

struct Counter;

impl Counts for Counter {
    fn count_of<T>(&self, rc: &Rc<T>) -> usize {
        Rc::strong_count(rc)
    }
}

struct Item {
    id: u32,
}

fn through<C: Counts>(counter: &C, rc: &Rc<Item>) -> usize {
    counter.count_of(rc)
}

struct Label {
    #[cfg_attr(rust_js, rust_js::test)]
    text: String,
    next: RefCell<Option<Rc<Label>>>,
}

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn shared_twice<T>(value: T) {
    let first = Rc::new(value);
    let second = Rc::clone(&first);
    drop(first);
    println!("one left");
    drop(second);
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Shown {
    Named(Rc<Slot<String>>),
    Labelled(Rc<Label>),
}

fn shown(shown: &Shown) -> String {
    match shown {
        Shown::Named(slot) => format!("named {}", slot.value),
        Shown::Labelled(label) => format!("label {}", label.text),
    }
}

fn main() {
    let slot = Rc::new(Slot { value: 3u32 });
    let again = Rc::clone(&slot);
    println!("{} {}", count_slot(&slot), again.value);
    let item = Rc::new(Item { id: 7 });
    let copies = [Rc::clone(&item), Rc::clone(&item)];
    println!("{} {}", through(&Counter, &item), copies.len());
    drop(copies);
    println!("{} {}", through(&Counter, &item), item.id);
    let named = Shown::Named(Rc::new(Slot {
        value: String::from("x"),
    }));
    let labelled = Shown::Labelled(Rc::new(Label {
        text: String::from("y"),
        next: RefCell::new(None),
    }));
    println!("{} {}", shown(&named), shown(&labelled));
    if let Shown::Labelled(label) = &labelled {
        println!("{}", label.next.borrow().is_none());
    }
    shared_twice(Loud(1));
    shared_twice(2u8);
}
