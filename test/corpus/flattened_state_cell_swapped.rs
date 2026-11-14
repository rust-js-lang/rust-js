// A class's flattened state's cell swapped with another, `RefCell::swap`:
// each moved out, a copy of its variant's keys, and written with the
// other's (#687, ADR 1044). Two classes' states, a state and a `RefCell`
// of its own, a parameter's and a variable's, either way round, each after
// a write of one variant over another, whose earlier keys the object keeps;
// and what's swapped in then changed, apart from what it came from.
use std::cell::RefCell;

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

fn bump(state: &mut Status) {
    if let Status::Pending { tries } = state {
        *tries += 10;
    }
}

fn exchange(task: &Task, other: &RefCell<Status>) {
    task.state.swap(other);
}

fn exchanged(other: &RefCell<Status>, task: &Task) {
    other.swap(&task.state);
}

fn main() {
    let a = Task::new(Status::Pending { tries: 1 }, "a");
    let b = Task::new(Status::Done { value: String::from("x") }, "b");
    a.state.swap(&b.state);
    println!("{}, {}", shown(&a), shown(&b));
    bump(&mut b.state.borrow_mut());
    println!("{}, {}", shown(&a), shown(&b));
    *a.state.borrow_mut() = Status::Halted;
    b.state.swap(&a.state);
    println!("{}, {}", shown(&a), shown(&b));

    let own = RefCell::new(Status::Pending { tries: 2 });
    exchange(&a, &own);
    println!("{}, {:?}", shown(&a), own.borrow());
    bump(&mut a.state.borrow_mut());
    println!("{}, {:?}", shown(&a), own.borrow());
    exchanged(&own, &b);
    println!("{}, {:?}", shown(&b), own.borrow());
    bump(&mut own.borrow_mut());
    println!("{}, {:?}", shown(&b), own.borrow());
}
