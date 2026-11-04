// A class's flattened state's `try_borrow` and `try_borrow_mut`, each held
// only while no other borrow can meet it: `Ok` of its guard, the handle on
// the class's keys, as no other borrow can be held (#687, ADR 1044); read,
// tested, written in a field and whole; beside a plain cell of the state
// whose borrows are counted.
use std::cell::RefCell;

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
    pub name: String,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Task {
    pub fn new(state: Status, name: &str) -> Self {
        Task {
            state: RefCell::new(state),
            name: name.to_string(),
        }
    }
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "task");
    match task.state.try_borrow() {
        Ok(state) => println!("read {:?}", *state),
        Err(error) => println!("read {error}"),
    }
    if let Ok(mut state) = task.state.try_borrow_mut()
        && let Status::Pending { tries } = &mut *state
    {
        *tries += 1;
    }
    println!("free {}", task.state.try_borrow_mut().is_ok());
    if let Ok(mut state) = task.state.try_borrow_mut() {
        *state = Status::Done {
            value: String::from("x"),
        };
    }
    match task.state.try_borrow() {
        Ok(state) => println!("{} {:?}", task.name, *state),
        Err(error) => println!("{error}"),
    }
    // A plain cell of the state, whose borrows are counted: the class's
    // state's guards still count nothing.
    let other = RefCell::new(Status::Pending { tries: 0 });
    let held = other.borrow();
    println!("other {}", other.try_borrow_mut().is_err());
    if let Ok(state) = task.state.try_borrow_mut() {
        println!("{} {:?} {:?}", task.name, *state, *held);
    }
}
