//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #2509 a `cloned()` whose steps aren't one line names its call's line
// A held `cloned()` asked `next()` and then `count()`, whose items std
// clones at two lines, `function.rs:250:5` and `166:5`: rust-js names the
// call's line until std's `Cloned` is compiled (#1458, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2), RefCell::new(3)];
    let _held = items[2].borrow_mut();
    let mut each = items.iter().cloned();
    println!("{:?}", each.next());
    println!("{}", each.count());
}
