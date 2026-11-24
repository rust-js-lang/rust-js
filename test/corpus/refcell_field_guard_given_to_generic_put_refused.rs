//@ compile-fail: a `RefCell` guard's `&mut`, `&mut std::cell::RefMut<'_, State>`, given to generic code's `&mut T`, which may write the guard whole
// A `RefCell` field's guard of a struct, a flattened state's, given by
// `&mut` to generic code that may write it whole, given a plain cell's
// guard: Rust's write makes the variable the other cell's guard; the JS
// would write the other's state into the field's object, in place, and
// never give the other's borrow back, so it's refused (#2475).
use std::cell::RefCell;

#[derive(Debug, Clone)]
struct State {
    count: u32,
}

struct Task {
    state: RefCell<State>,
}

fn put<T>(slot: &mut T, v: T) {
    *slot = v;
}

fn main() {
    let task = Task { state: RefCell::new(State { count: 1 }) };
    let other = RefCell::new(State { count: 2 });
    {
        let mut state = task.state.borrow_mut();
        put(&mut state, other.borrow_mut());
        state.count += 10;
        println!("{} {}", task.state.try_borrow_mut().is_ok(), other.try_borrow_mut().is_ok());
    }
    println!("{:?} {:?}", task.state.borrow(), other.borrow());
}
