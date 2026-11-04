//@ run-fail: RefCell already borrowed
// A class's flattened state's cell swapped with a plain `RefCell` of its
// state, borrowed while it's swapped: Rust's `borrow_mut` of the plain one
// panics, and so does the JS, which checks that one's borrows, as they're
// counted, though the state's are not (#687, ADR 1044).
use std::cell::RefCell;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "halted")]
    Halted,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Task {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<Status>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Task {
    pub fn new(state: Status) -> Self {
        Task {
            state: RefCell::new(state),
        }
    }
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 });
    let other = RefCell::new(Status::Halted);
    task.state.swap(&other);
    println!("swapped {:?} {:?}", task.state.borrow(), other.borrow());
    let held = other.borrow();
    task.state.swap(&other);
    println!("swapped again {:?}", *held);
}
