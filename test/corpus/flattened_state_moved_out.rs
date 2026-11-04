// What's moved out of a class's flattened state whole, `mem::replace`,
// `mem::take`, `mem::swap` and `Cell::from_mut(..).replace`: a copy of its
// variant's keys, apart from the class's object, which the write then
// changes (#658, #659, ADR 1044). Moved out through its guard, held by a
// `let` (#659), and by a function it's lent to (#658), each after a write
// of one variant over another, whose earlier keys the object keeps; and a
// state that's no class's, a variable of its own, lent the same; and its
// cell's own `replace`.
use std::cell::{Cell, RefCell};
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

fn finish(state: &mut Status) -> Status {
    mem::replace(state, Status::Done { value: String::from("v") })
}

fn halt(state: &mut Status) -> Status {
    mem::take(state)
}

fn exchange(state: &mut Status, other: &mut Status) {
    mem::swap(state, other);
}

fn restart(state: &mut Status) -> Status {
    Cell::from_mut(state).replace(Status::Pending { tries: 0 })
}

fn bump(old: &mut Status) {
    if let Status::Pending { tries } = old {
        *tries += 10;
    }
}

fn shown(task: &Task) -> String {
    format!("{} {:?}", task.name, task.state.borrow())
}

fn lent() {
    let task = Task::new(Status::Pending { tries: 1 }, "lent");
    let mut old = finish(&mut task.state.borrow_mut());
    bump(&mut old);
    println!("finish {old:?}, {}", shown(&task));
    let old = halt(&mut task.state.borrow_mut());
    println!("halt {old:?}, {}", shown(&task));
    let mut other = Status::Pending { tries: 2 };
    exchange(&mut task.state.borrow_mut(), &mut other);
    bump(&mut other);
    println!("exchange {other:?}, {}", shown(&task));
    let old = restart(&mut task.state.borrow_mut());
    println!("restart {old:?}, {}", shown(&task));
    let mut old = finish(&mut task.state.borrow_mut());
    bump(&mut old);
    println!("finish {old:?}, {}", shown(&task));
}

fn held() {
    let task = Task::new(Status::Pending { tries: 3 }, "held");
    {
        let mut state = task.state.borrow_mut();
        let mut old = mem::replace(&mut *state, Status::Done { value: String::from("w") });
        bump(&mut old);
        println!("replace {old:?}");
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let old = mem::take(&mut *state);
        println!("take {old:?}");
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let mut other = Status::Pending { tries: 4 };
        mem::swap(&mut other, &mut *state);
        println!("swap {other:?}");
        bump(&mut state);
    }
    println!("{}", shown(&task));
    {
        let mut state = task.state.borrow_mut();
        let mut old = Cell::from_mut(&mut *state).replace(Status::Done { value: String::from("x") });
        bump(&mut old);
        println!("cell {old:?}");
    }
    println!("{}", shown(&task));
}

fn its_cell() {
    let task = Task::new(Status::Pending { tries: 6 }, "cell");
    let mut old = task.state.replace(Status::Done { value: String::from("y") });
    bump(&mut old);
    println!("replace {old:?}, {}", shown(&task));
    let old = task.state.replace(Status::Pending { tries: 7 });
    println!("replace {old:?}, {}", shown(&task));
}

fn own() {
    let mut state = Status::Pending { tries: 5 };
    let mut old = finish(&mut state);
    bump(&mut old);
    println!("own {old:?} {state:?}");
    let old = halt(&mut state);
    println!("own {old:?} {state:?}");
}

fn main() {
    lent();
    held();
    its_cell();
    own();
}
