// A field's `RefCell` only ever borrowed for a moment is the value it
// holds, one with a destructor too (ADRs 0362, 0863): written through
// `get_mut`, replaced and taken, each old value dropped as it goes, and
// dropped with what holds it, a variable, a `Vec`'s item, an `Rc`'s last
// owner, as native Rust drops each.

use std::cell::RefCell;
use std::rc::Rc;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Slot {
    held: RefCell<Option<Loud>>,
}

fn fill(slot: &Slot, name: &'static str) -> bool {
    slot.held.replace(Some(Loud(name))).is_some()
}

fn main() {
    let mut slot = Slot {
        held: RefCell::new(Some(Loud("a"))),
    };
    *slot.held.get_mut() = Some(Loud("b"));
    println!("set {}", slot.held.borrow().is_some());
    println!("filled {}", fill(&slot, "c"));
    let taken = slot.held.take();
    println!("taken {}", taken.is_some());
    drop(taken);
    println!("empty {}", slot.held.borrow().is_none());
    fill(&slot, "d");
    {
        let slots = vec![
            Slot {
                held: RefCell::new(Some(Loud("e"))),
            },
            Slot {
                held: RefCell::new(None),
            },
        ];
        println!("slots {}", slots.len());
    }
    let shared = Rc::new(Slot {
        held: RefCell::new(Some(Loud("f"))),
    });
    let other = Rc::clone(&shared);
    println!("owners {}", Rc::strong_count(&shared));
    drop(shared);
    println!("one left");
    drop(other);
    println!("end");
}
