// Pointers into what a `RefCell`, a `Mutex` and an `Option` hold, compared
// and read through but no address asked: a crate that asks none records
// nothing of where they keep it, no `$storedValue`, `$stored` or
// `$storedIn` (#1114), beside `address_of_what_a_cell_lends`, which asks.
use std::cell::RefCell;
use std::sync::Mutex;

#[derive(Debug)]
struct P {
    a: u8,
    b: u32,
}

struct Held(P);

trait Measured {
    fn size(&self) -> u32;
}

impl Measured for P {
    fn size(&self) -> u32 {
        u32::from(self.a) + self.b
    }
}

struct Counter {
    count: RefCell<u32>,
}

fn main() {
    let cell = RefCell::new(P { a: 1, b: 2 });
    let first = &raw const *cell.borrow();
    let second = &raw const *cell.borrow();
    println!("cell {}", std::ptr::eq(first, second));
    let lock = Mutex::new(P { a: 3, b: 4 });
    let guarded = &raw const lock.lock().unwrap().b;
    println!("mutex {}", unsafe { *guarded });
    let o = Some(P { a: 5, b: 6 });
    let p = o.as_ref().unwrap() as *const P;
    println!("option {}", std::ptr::eq(p, o.as_ref().unwrap()));
    // A `dyn`'s pointer made thin is the field's it was made of.
    let held = Held(P { a: 7, b: 8 });
    let shown: *const dyn Measured = &held.0;
    let thin = shown as *const P;
    println!("thin {} {}", thin == &raw const held.0, unsafe { (*shown).size() });
    // An item's pointer is stepped along its array.
    let items = [P { a: 9, b: 10 }, P { a: 11, b: 12 }];
    let first = &raw const items[0];
    println!("stepped {}", unsafe { (*first.add(1)).a });
    let counter = Counter { count: RefCell::new(7) };
    *counter.count.borrow_mut() += 1;
    println!("{:?} {:?} {}", cell.borrow(), o, counter.count.borrow());
}
