// A `bool`'s `toggle()`, Rust 1.100's: its place given its negation,
// `b = !b`, of a variable, a `&mut` parameter's handle, a field, a tuple
// field, a `Vec`'s item through `iter_mut()` and by index, a `RefCell`'s
// guard, a closure's capture, a `fn` given to `for_each` and an `Option`'s
// payload, as native's (#1656).
use std::cell::{Cell, RefCell};
struct S {
    on: bool,
    pair: (bool, u8),
}
fn flip(b: &mut bool) {
    b.toggle();
}
fn flip_twice(b: &mut bool) -> bool {
    b.toggle();
    b.toggle();
    *b
}
fn main() {
    let mut a = false;
    a.toggle();
    println!("{a}");
    flip(&mut a);
    println!("{a} {}", flip_twice(&mut a));
    let mut s = S {
        on: true,
        pair: (false, 1),
    };
    s.on.toggle();
    s.pair.0.toggle();
    println!("{} {} {}", s.on, s.pair.0, s.pair.1);
    let mut v = vec![true, false, true];
    for b in v.iter_mut() {
        b.toggle();
    }
    v[1].toggle();
    println!("{v:?}");
    let r = RefCell::new(false);
    r.borrow_mut().toggle();
    println!("{}", r.borrow());
    let mut c = true;
    let mut f = || c.toggle();
    f();
    f();
    f();
    println!("{c}");
    let cell = Cell::new(true);
    let mut x = cell.get();
    x.toggle();
    cell.set(x);
    println!("{}", cell.get());
    let mut arr = [false; 3];
    arr.iter_mut().for_each(bool::toggle);
    println!("{arr:?}");
    let mut o = Some(false);
    if let Some(b) = o.as_mut() {
        b.toggle();
    }
    println!("{o:?}");
}
