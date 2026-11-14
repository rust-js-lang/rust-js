// A class's flattened state swapped with another place, `mem::swap`: a
// struct's field and a `Vec`'s item, through its guard
// and lent to a function. Each side's value is moved out apart, a
// flattened state's a copy of its variant's keys, and written on the
// object as its keys, the class's `name` kept (#659, ADR 1044).
use std::cell::RefCell;
use std::mem;

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

#[derive(Debug)]
struct Holder {
    f: Status,
    items: Vec<Status>,
}

fn swap_two(a: &mut Status, b: &mut Status) {
    mem::swap(a, b);
}

fn main() {
    let task = Task::new(Status::Pending { tries: 1 }, "t");
    let mut h = Holder {
        f: Status::Done { value: String::from("f") },
        items: vec![Status::Halted, Status::Pending { tries: 9 }],
    };
    {
        let mut state = task.state.borrow_mut();
        mem::swap(&mut *state, &mut h.f);
    }
    println!("{:?} {} {h:?}", task.state.borrow(), task.name);
    {
        let mut state = task.state.borrow_mut();
        mem::swap(&mut h.items[1], &mut *state);
    }
    println!("{:?} {} {h:?}", task.state.borrow(), task.name);
    swap_two(&mut task.state.borrow_mut(), &mut h.f);
    println!("{:?} {} {h:?}", task.state.borrow(), task.name);
}
