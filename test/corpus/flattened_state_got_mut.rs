// A class's flattened state's `get_mut`: the `&mut` rustc holds alone is
// its handle on the class's keys, written in a field, whole, moved out of
// with `mem::replace`, and lent to a function that writes it whole (#687,
// ADR 1044); the class's other keys kept.
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

fn finish(state: &mut Status, value: &str) {
    *state = Status::Done {
        value: value.to_string(),
    };
}

fn main() {
    let mut task = Task::new(Status::Pending { tries: 1 }, "task");
    if let Status::Pending { tries } = task.state.get_mut() {
        *tries += 1;
    }
    println!("{}", shown(&task));
    *task.state.get_mut() = Status::Done {
        value: String::from("a"),
    };
    println!("{}", shown(&task));
    *task.state.get_mut() = Status::Pending { tries: 3 };
    let mut old = mem::replace(task.state.get_mut(), Status::Halted);
    if let Status::Pending { tries } = &mut old {
        *tries += 10;
    }
    println!("{old:?}, {}", shown(&task));
    finish(task.state.get_mut(), "b");
    println!("{}", shown(&task));
    let state = task.state.get_mut();
    if let Status::Done { value } = state {
        value.push('!');
    }
    *state = Status::Pending { tries: 4 };
    println!("{}", shown(&task));
}
