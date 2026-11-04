//@ run-fail: RefCell already borrowed
// A class's flattened state's cell swapped with itself, through two
// references to one class: Rust's second `borrow_mut` panics, and so does
// the JS, which tells the two apart by the object they're on, as the state
// counts no borrows (#687, ADR 1044).
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

fn exchange(a: &Task, b: &Task) {
    a.state.swap(&b.state);
    println!("swapped {:?} {:?}", a.state.borrow(), b.state.borrow());
}

fn main() {
    let a = Task::new(Status::Pending { tries: 1 });
    let b = Task::new(Status::Halted);
    exchange(&a, &b);
    exchange(&a, &a);
}
