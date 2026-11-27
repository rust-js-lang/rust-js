//@ run-fail: RefCell already mutably borrowed
// A `cloned()`'s `count()` of `Option`s of a `RefCell` mutably borrowed
// panics where the `Option`'s own clone clones it, `option.rs:2280:31`,
// never at `Cloned::fold`'s line of the item it clones, which is the
// `Option`, not the cell (#2509).
use std::cell::RefCell;

fn main() {
    let items = vec![Some(RefCell::new(1u8)), None, Some(RefCell::new(2))];
    println!("{}", items.iter().cloned().count());
    let _held = items[2].as_ref().unwrap().borrow_mut();
    println!("{}", items.iter().cloned().count());
}
