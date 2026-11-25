//@ compile-fail: a flattened state lent, or borrowed where another borrow of it may be
// A class's flattened state's `try_borrow` guard taken out of its `Ok` and
// kept past the `Result`'s drop, `let state = match .. { Ok(state) =>
// state, .. };`, is held until its own drop (#2490's flip: a payload's
// move is a move out of the `Result`). A borrow of the state while it's
// held, which Rust finds busy, `true`, refuses the flattened state, which
// can't count (ADR 1044); taken to end at the `Result`'s drop, it
// compiled, and found it free.
use std::cell::RefCell;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
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
    let state = match task.state.try_borrow() {
        Ok(state) => state,
        Err(_) => return,
    };
    let busy = task.state.try_borrow_mut().is_err();
    println!("{:?} {busy}", *state);
}
