//@ run-fail: RefCell already borrowed
// A guard of one cell held across another's `replace_with` whose function
// borrows the first mutably: Rust's `borrow_mut` panics, and so does the
// JS, as a cell's own call runs what its bounds say, the function, while
// the guard is held (#687). The guard was quiet, as `replace_with` ran
// nothing but its own borrow, so the first cell was uncounted, and the JS
// wrote it. The cell replaced is generic code's, of no class its flow
// says.
use std::cell::RefCell;

fn bump<T: Clone>(cell: &RefCell<T>, b: &RefCell<String>) -> T {
    let held = b.borrow();
    let old = cell.replace_with(|n| {
        b.borrow_mut().push('!');
        n.clone()
    });
    println!("{}", *held);
    old
}

fn main() {
    let cells = vec![RefCell::new(1u32), RefCell::new(2)];
    let b = RefCell::new(String::from("b"));
    let old = bump(&cells[1], &b);
    println!("{old} {}", cells[1].borrow());
}
