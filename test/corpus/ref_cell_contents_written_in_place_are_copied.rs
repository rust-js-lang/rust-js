// `RefCell::new(a)` of a `Copy` struct, a cell that's its object, and
// `replace`, `swap` and `replace_with` writing it in place: `a` and the
// copies read out of the cell before keep their fields, as what a `RefCell`
// holds is changed through `&` (#1665).
#[derive(Clone, Copy, Default, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

use std::cell::RefCell;

fn main() {
    let a = Pair { a: 1, b: 2 };
    let cell = RefCell::new(a);
    let before = *cell.borrow();
    cell.replace(Pair { a: 5, b: 6 });
    println!("{a:?} {before:?} {:?}", cell.borrow());
    let other = RefCell::new(Pair { a: 7, b: 8 });
    let snapshot = *other.borrow();
    cell.swap(&other);
    println!("{snapshot:?} {:?} {:?}", cell.borrow(), other.borrow());
    let taken = cell.replace_with(|old| Pair { a: old.a + 1, b: old.b + 1 });
    println!("{snapshot:?} {taken:?} {:?}", cell.borrow());
}
