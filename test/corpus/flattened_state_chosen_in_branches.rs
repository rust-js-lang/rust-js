// A class's flattened state's `get_mut()` chosen in two branches, `let
// state = if first { a.state.get_mut() } else { b.state.get_mut() }`, and
// lent to `Cell::from_mut` and as a `&mut dyn`: the variable is written
// twice, so no one handle is known to be what it holds, and each lend sets
// the keys of the object the branch taken chose (#2429, ADR 0791).
use std::cell::{Cell, RefCell};

#[derive(Debug, Default)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
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

trait Set<T> {
    fn set(&mut self, value: T);
}

impl<T> Set<T> for T {
    fn set(&mut self, value: T) {
        *self = value;
    }
}

fn finished(state: &mut dyn Set<Status>, tries: u32) {
    state.set(Status::Pending { tries });
}

fn shown(task: &Task) -> String {
    format!("{} {:?}", task.name, task.state.borrow())
}

fn pick(a: &mut Task, b: &mut Task, first: bool) {
    let state = if first { a.state.get_mut() } else { b.state.get_mut() };
    halt(Cell::from_mut(state));
}

fn chosen(a: &mut Task, b: &mut Task, first: bool) {
    let state = if first { a.state.get_mut() } else { b.state.get_mut() };
    finished(state, 9);
}

fn main() {
    let mut a = Task::new(Status::Pending { tries: 1 }, "a");
    let mut b = Task::new(Status::Pending { tries: 2 }, "b");
    pick(&mut a, &mut b, true);
    println!("{} {}", shown(&a), shown(&b));
    let mut c = Task::new(Status::Pending { tries: 3 }, "c");
    let mut d = Task::new(Status::Pending { tries: 4 }, "d");
    pick(&mut c, &mut d, false);
    println!("{} {}", shown(&c), shown(&d));
    chosen(&mut a, &mut b, true);
    println!("{} {}", shown(&a), shown(&b));
    chosen(&mut c, &mut d, false);
    println!("{} {}", shown(&c), shown(&d));
}
