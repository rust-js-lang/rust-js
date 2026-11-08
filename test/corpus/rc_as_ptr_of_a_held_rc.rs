// `Rc::as_ptr` of an `Rc` a tuple struct's field holds, `Rc::as_ptr(&held.0)`:
// a pointer to what the `Rc` points at, the object its clones share, not to
// the array item that holds the `Rc`, `$ptr(held, 0)`, which made two clones'
// pointers differ (#1019).
use std::rc::Rc;

struct Update {
    lane: u32,
}

struct Held<T>(T);

fn main() {
    let first = Held(Rc::new(Update { lane: 1 }));
    let second = Held(Rc::new(Update { lane: 1 }));
    let again = Held(Rc::clone(&first.0));
    println!(
        "{} {}",
        Rc::as_ptr(&first.0) == Rc::as_ptr(&again.0),
        Rc::as_ptr(&first.0) == Rc::as_ptr(&second.0)
    );
    let pair = (Rc::clone(&first.0), Rc::clone(&second.0));
    println!(
        "{} {} {}",
        Rc::as_ptr(&pair.0) == Rc::as_ptr(&first.0),
        Rc::as_ptr(&pair.1) == Rc::as_ptr(&first.0),
        unsafe { (*Rc::as_ptr(&pair.1)).lane }
    );
}
