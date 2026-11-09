//@ run-pass
// A raw pointer to a nested place whose root is a `Ref`'s, `(*cell.borrow())[2].a`,
// read through, in a crate that asks no address: its root, the
// temporary the `Ref` gives, is made first, never named where no JS
// declares it (#1110).
use std::cell::RefCell;
use std::rc::Rc;

struct P {
    a: u8,
    b: u32,
}

fn main() {
    let cell = RefCell::new([P { a: 1, b: 10 }, P { a: 2, b: 20 }, P { a: 3, b: 30 }]);
    let a = &raw const (*cell.borrow())[2].a;
    let b = &raw const (*cell.borrow())[1].b;
    unsafe {
        println!("{} {}", *a, *b);
    }
    println!("{}", cell.borrow()[1].b);
    // Through an `Rc` too, read where it's made.
    let items = Rc::new(RefCell::new([P { a: 4, b: 40 }, P { a: 5, b: 50 }]));
    println!("{}", unsafe { *&raw const (*items.borrow())[1].a });
}
