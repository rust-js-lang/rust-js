// A class's flattened state's `&mut` lent on where its handle is a
// variable's only through what holds it: `try_borrow_mut()`'s `Ok`, whose
// guard is given to `Cell::from_mut` and, reborrowed, to a function that
// gives the `&mut` back; and `get_mut()`'s given as a `&mut dyn`, to
// `Cell::from_mut` and to that function. Each is the guard's handle, whose
// setter sets each key on the class's object, never one assigning the
// variable holding it (#2429, ADR 0791).
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

trait Set<T> {
    fn set(&mut self, value: T);
}

impl<T> Set<T> for T {
    fn set(&mut self, value: T) {
        *self = value;
    }
}

fn finished(state: &mut dyn Set<Status>, value: &str) {
    state.set(Status::Done {
        value: value.to_string(),
    });
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

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "task");
    if let Ok(mut state) = task.state.try_borrow_mut() {
        halt(Cell::from_mut(&mut *state));
    }
    println!("{}", shown(&task));
    if let Ok(mut state) = task.state.try_borrow_mut() {
        *kept(&mut state) = Status::Pending { tries: 7 };
    }
    println!("{}", shown(&task));
    let mut owned = Task::new(Status::Halted, "owned");
    finished(owned.state.get_mut(), "c");
    println!("{}", shown(&owned));
    halt(Cell::from_mut(owned.state.get_mut()));
    println!("{}", shown(&owned));
    *kept(owned.state.get_mut()) = Status::Pending { tries: 9 };
    println!("{}", shown(&owned));
}
