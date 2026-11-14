// A class's flattened state's `replace_with(f)`: `f` given a `&mut` to it,
// its handle on the class's keys, which it writes through; what it held
// then a copy of its variant's keys, apart from the class's object, and
// what `f` made written there (#687, ADR 1044). Its borrow, held while `f`
// runs, counts nothing, as no borrow `f` makes can meet it. And a guard of
// another cell held across a `replace_with` whose function borrows that
// cell, which `try_borrow_mut` sees, as Rust's does.
use std::cell::RefCell;
use std::mem;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
    #[cfg_attr(rust_js, rust_js::name = "halted")]
    Halted,
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

fn shown(task: &Task) -> String {
    format!("{} {:?}", task.name, task.state.borrow())
}

fn advance(state: &mut Status) -> Status {
    match state {
        Status::Pending { tries } => {
            *tries += 1;
            Status::Done {
                value: tries.to_string(),
            }
        }
        _ => Status::Halted,
    }
}

fn held_across() {
    let cell = RefCell::new(1u32);
    let other = RefCell::new(5u32);
    let held = other.borrow();
    let old = cell.replace_with(|n| {
        let free = other.try_borrow_mut().is_ok();
        if free { *n + 10 } else { *n + 20 }
    });
    println!("held {old} {} {}", cell.borrow(), *held);
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "task");
    let mut old = task.state.replace_with(advance);
    if let Status::Pending { tries } = &mut old {
        *tries += 10;
    }
    println!("{old:?}, {}", shown(&task));
    let old = task.state.replace_with(|state| match state {
        Status::Done { value } => {
            value.push('!');
            Status::Pending { tries: 7 }
        }
        _ => Status::Halted,
    });
    println!("{old:?}, {}", shown(&task));
    let old = task.state.replace_with(|state| mem::replace(state, Status::Halted));
    println!("{old:?}, {}", shown(&task));
    held_across();
}
