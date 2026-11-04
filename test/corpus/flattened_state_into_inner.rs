// A class's flattened state moved out of its class, `into_inner`: a copy of
// its variant's keys, apart from the class's object, whose other fields
// are still its own (#687, ADR 1044); after a write of one variant over
// another, whose earlier keys the object keeps.
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

fn finished(task: Task) -> Status {
    task.state.into_inner()
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "a");
    *task.state.borrow_mut() = Status::Done {
        value: String::from("x"),
    };
    println!("{:?}", finished(task));

    let mut task = Task::new(Status::Pending { tries: 2 }, "b");
    let mut state = task.state.into_inner();
    task.name.push('!');
    if let Status::Pending { tries } = &mut state {
        *tries += 10;
    }
    println!("{state:?}, {}", task.name);

    let task = Task::new(Status::Halted, "c");
    println!("{:?}", task.state.into_inner());
}
