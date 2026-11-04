// A `&mut` to a class's flattened state its guard lends, handed on:
// `Cell::from_mut(&mut *state)`'s cell swapped, taken, set and given to a
// function, and the `&mut` given to a function that gives it back. Each is
// the guard's handle, whose setter sets each key on the class's object,
// never one assigning the variable holding it; what's moved out, a copy of
// its variant's keys (#659, ADR 1044).
use std::cell::{Cell, RefCell};

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

fn swapped(state: &mut Status, other: &mut Status) {
    Cell::from_mut(state).swap(Cell::from_mut(other));
}

fn shown(task: &Task) -> String {
    format!("{} {:?}", task.name, task.state.borrow())
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "t");
    let mut other = Status::Done { value: String::from("o") };
    {
        let mut state = task.state.borrow_mut();
        Cell::from_mut(&mut *state).swap(Cell::from_mut(&mut other));
    }
    println!("swap {}, {other:?}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let old = Cell::from_mut(&mut *state).take();
        println!("take {old:?}");
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        Cell::from_mut(&mut *state).set(Status::Pending { tries: 2 });
    }
    println!("set {}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        halt(Cell::from_mut(&mut *state));
    }
    println!("given {}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let r = kept(&mut *state);
        *r = Status::Done { value: String::from("kept") };
    }
    println!("kept {}", shown(&task));
    swapped(&mut task.state.borrow_mut(), &mut other);
    println!("lent {}, {other:?}", shown(&task));
}
