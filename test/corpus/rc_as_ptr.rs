// `Rc::as_ptr` (ADR 0562, amended): a pointer to what the `Rc` points at,
// so two compare as the `Rc`s do, `a === b`, as `Rc::ptr_eq` does; of a
// counted `Rc`'s, its value's place.
use std::rc::Rc;
use std::sync::Arc;

struct Update {
    lane: u32,
}

struct Point {
    x: u32,
    y: u32,
}

fn differ(a: &Option<Rc<Update>>, b: &Option<Rc<Update>>) -> bool {
    a.as_ref().map(Rc::as_ptr) != b.as_ref().map(Rc::as_ptr)
}

fn main() {
    let first = Rc::new(Update { lane: 1 });
    let second = Rc::new(Update { lane: 1 });
    let again = Rc::clone(&first);
    println!("{} {}", Rc::as_ptr(&first) == Rc::as_ptr(&again), Rc::as_ptr(&first) == Rc::as_ptr(&second));
    let some = Some(Rc::clone(&first));
    println!(
        "{} {} {} {}",
        differ(&some, &Some(again)),
        differ(&some, &Some(second)),
        differ(&some, &None),
        differ(&None, &None)
    );
    // Read through it, as `&*` of it is what the `Rc` points at.
    let lane = unsafe { (*Rc::as_ptr(&first)).lane };
    println!("{lane}");

    // A counted `Rc` of text, its value's place.
    let name = Rc::new(String::from("fiber"));
    let other = Rc::new(String::from("fiber"));
    let shared = Rc::clone(&name);
    println!("{} {}", Rc::strong_count(&name), Rc::as_ptr(&name) == Rc::as_ptr(&shared));
    println!("{}", Rc::as_ptr(&name) == Rc::as_ptr(&other));
    println!("{}", unsafe { &*Rc::as_ptr(&name) }.len());

    // A number's `Rc`, counted for its identity, as `Rc::ptr_eq` counts it.
    let n = Rc::new(5u32);
    let m = Rc::clone(&n);
    let k = Rc::new(5u32);
    println!("{} {} {}", Rc::as_ptr(&n) == Rc::as_ptr(&m), Rc::as_ptr(&n) == Rc::as_ptr(&k), *n + *k);

    // Two fields' places, the same where they're one field of one object.
    let point = Point { x: 1, y: 1 };
    println!(
        "{} {} {}",
        std::ptr::eq(&raw const point.x, &raw const point.y),
        std::ptr::eq(&raw const point.x, &raw const point.x),
        std::ptr::eq(&point.x, &point.y)
    );

    let a = Arc::new(Update { lane: 2 });
    let b = Arc::clone(&a);
    println!("{} {}", Arc::as_ptr(&a) == Arc::as_ptr(&b), a.lane + b.lane);
}
