// A variable given what's made where it's made, `listeners = []`, though
// a closure captures it, where that closure is made only after it's
// given: nothing could call it before (rd105, ADR 0853). One made before
// the `if` it's given in, called between, reads what the variable was.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type Listeners = &'static RefCell<Vec<u32>>;

fn listeners_of(map: &RefCell<HashMap<u32, Listeners>>, key: u32) -> Option<Listeners> {
    map.borrow().get(&key).copied()
}

fn set_listeners(map: &RefCell<HashMap<u32, Listeners>>, key: u32, listeners: Listeners) {
    map.borrow_mut().insert(key, listeners);
}

fn subscribe(map: &RefCell<HashMap<u32, Listeners>>, key: u32, listener: u32) -> impl Fn() + use<> {
    let listeners = match listeners_of(map, key) {
        Some(listeners) => listeners,
        None => {
            let listeners: Listeners = Box::leak(Box::new(RefCell::new(Vec::new())));
            set_listeners(map, key, listeners);
            listeners
        }
    };
    listeners.borrow_mut().push(listener);
    move || listeners.borrow_mut().retain(|&l| l != listener)
}

fn before_around(n: u32) -> u32 {
    let seen = Cell::new(1);
    let read = || seen.get();
    if n > 0 {
        seen.set({
            let seen = n * 10;
            println!("{}", read());
            seen
        });
    }
    seen.get()
}

pub fn main() {
    let map = RefCell::new(HashMap::new());
    let a = subscribe(&map, 1, 10);
    let b = subscribe(&map, 1, 20);
    let c = subscribe(&map, 2, 30);
    a();
    println!("{:?}", listeners_of(&map, 1).map(|l| l.borrow().clone()));
    b();
    c();
    println!("{:?} {:?}", listeners_of(&map, 1).map(|l| l.borrow().len()), listeners_of(&map, 2).map(|l| l.borrow().len()));
    println!("{}", before_around(4));
}
