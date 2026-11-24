//@ compile-fail: a `RefCell` guard written whole by a closure that borrows its variable
// A flattened state's guard (ADR 1044), which counts nothing, written
// whole by a closure with a plain cell's counted guard is refused, as any
// guard so written is (#2475, m1-lisa's probe): the plain cell was never
// released, and its next borrow panicked where Rust prints
// `Pending { tries: 0 } Pending { tries: 9 }`.
use std::cell::{RefCell, RefMut};

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Task {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<Status>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Task {
    pub fn new(state: Status) -> Self {
        Task { state: RefCell::new(state) }
    }
}

fn kept<'a>(g: &'a mut RefMut<'_, Status>) -> &'a mut Status {
    &mut **g
}

fn main() {
    let a = Task::new(Status::Pending { tries: 0 });
    let plain = RefCell::new(Status::Done { value: "p".into() });
    {
        let mut state = a.state.borrow_mut();
        let other = plain.borrow_mut();
        let swap_in = || state = other;
        swap_in();
        *kept(&mut state) = Status::Pending { tries: 9 };
    }
    println!("{:?} {:?}", a.state.borrow(), plain.borrow());
    if let Status::Pending { tries } = &*a.state.borrow() {
        println!("{tries}");
    }
    if let Status::Done { value } = &*plain.borrow() {
        println!("{value}");
    }
}
