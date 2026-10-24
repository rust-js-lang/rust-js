// One object two closures share, as React's `lazy` shares its payload with
// the callbacks its thenable settles: an `Rc` of `Cell`s, a clone moved
// into each, which is the same JS object, so each reads it by its name,
// `payload.status = 1`, with no `const resolved = payload` (ADR 0846):
// where nothing gives the parameter another value.

use std::cell::Cell;
use std::rc::Rc;

struct Payload {
    status: Cell<i32>,
    result: Cell<f64>,
}

fn settle(payload: Rc<Payload>, fulfil: bool, value: f64) {
    let resolved = payload.clone();
    let rejected = payload.clone();
    let on_resolve = move |value: f64| {
        resolved.status.set(1);
        resolved.result.set(value);
    };
    let on_reject = move |error: f64| {
        rejected.status.set(2);
        rejected.result.set(-error);
    };
    if fulfil {
        on_resolve(value);
    } else {
        on_reject(value);
    }
    println!("{} {}", payload.status.get(), payload.result.get());
}

// A parameter given another value after its clone is made: the clone is
// still of the first, so it keeps its name.
fn swapped(mut payload: Rc<Payload>, other: Rc<Payload>) {
    let first = payload.clone();
    payload = other;
    let mark = move || first.status.set(9);
    mark();
    println!("{}", payload.status.get());
}

fn main() {
    let payload = Rc::new(Payload {
        status: Cell::new(-1),
        result: Cell::new(0.0),
    });
    settle(payload.clone(), true, 2.5);
    println!("{} {}", payload.status.get(), payload.result.get());
    settle(payload.clone(), false, 4.0);
    println!("{} {}", payload.status.get(), payload.result.get());
    let other = Rc::new(Payload {
        status: Cell::new(0),
        result: Cell::new(0.0),
    });
    swapped(payload.clone(), other.clone());
    println!("{} {}", payload.status.get(), other.status.get());
}
