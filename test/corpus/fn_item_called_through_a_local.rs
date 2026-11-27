//@ run-fail: RefCell already borrowed
// A function item held in a local, `let t = touch;`, and called through it
// is a call of the function itself, as rustc's optimized MIR says (#2629):
// a fn item's type is the function, so its value is nothing more. A
// `#[track_caller]` function called so is given the call's line, not its
// own, and a cell the function borrows while a borrow of it is held is
// checked, so the second borrow panics as it does called by name; through
// a `fn` pointer, which may be any function, the line is the pointer's
// shim's, as std's.
use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;

#[track_caller]
fn line() -> u32 {
    Location::caller().line()
}

fn touch(cell: &Rc<RefCell<Vec<i32>>>) {
    cell.borrow_mut().push(2);
}

fn main() {
    println!("by name {}", line());
    let held_line = line;
    println!("through a local {}", held_line());
    let pointer: fn() -> u32 = line;
    println!("through a pointer {}", pointer());

    let made = Rc::new(RefCell::new(vec![1]));
    let other = Rc::new(RefCell::new(vec![7]));
    let t = touch;
    t(&other);
    println!("{:?}", other.borrow());
    let lent = &t;
    lent(&other);
    let held = made.borrow();
    println!("{}", held.len());
    t(&made);
    println!("{}", held.len());
}
