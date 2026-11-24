// A class's flattened state's guard whose `&mut` is given a name, `let r =
// &mut state;`, and only ever given to `deref_mut`: as `kept(r)`, `**r =
// ..`, `&mut **r` and through another name, `let again = r;`. Each is the
// guard's handle, whose setter sets each key on the class's object, never
// one assigning the variable holding it (#2443, ADR 0791). Read through,
// `tries(r)` and `println!("{:?}", **r)` borrowing it shared, it's still
// the handle, in a branch too (the metamorphic net's `emi-insert`); given
// to a function as itself, `touch(r)`, it's the guard's `&mut`, whose
// handle is on it.
use std::cell::{Cell, RefCell, RefMut};

#[derive(Debug, Default)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
    #[default]
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

fn halt(cell: &Cell<Status>) {
    cell.set(Status::Halted);
}

fn kept(state: &mut Status) -> &mut Status {
    state
}

fn shown(task: &Task) -> String {
    format!("{} {:?}", task.name, task.state.borrow())
}

fn touch(guard: &mut RefMut<'_, Status>) {
    **guard = Status::Halted;
}

fn tries(guard: &RefMut<'_, Status>) -> u32 {
    match **guard {
        Status::Pending { tries } => tries,
        _ => 0,
    }
}

fn retried(task: &Task, verbose: bool, tries: u32) {
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        if verbose {
            println!("was {:?}", **r);
        }
        *kept(r) = Status::Pending { tries };
    }
    println!("{}", shown(task));
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "task");
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        *kept(r) = Status::Pending { tries: 3 };
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        let again = r;
        *kept(again) = Status::Pending { tries: 4 };
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        **r = Status::Pending { tries: 5 };
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        touch(r);
        *kept(r) = Status::Pending { tries: 6 };
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        let next = tries(r) + 1;
        *kept(r) = Status::Pending { tries: next };
    }
    println!("{}", shown(&task));
    retried(&task, false, 7);
    retried(&task, true, 8);
    {
        let mut state = task.state.borrow_mut();
        let mut other = Status::Pending { tries: 8 };
        let r = &mut state;
        std::mem::swap(&mut **r, &mut other);
        println!("{:?}", other);
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut state;
        halt(Cell::from_mut(&mut **r));
    }
    println!("{}", shown(&task));
}
