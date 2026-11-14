// A class's flattened state's cell's own `take`, of a `Default` of the
// crate's: std's `RefCell::take` is `self.replace(Default::default())`, so
// `Default` runs before the cell is borrowed, and what's moved out is what
// the cell holds once it has run. A derived `Default`, and a written one
// that borrows the same state, read and written, through a thread-local:
// no borrow of it is held while it runs, so neither panics (#699, ADR 1044).
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Default)]
#[cfg_attr(rust_js, rust_js::tag = "kind")]
pub enum Plain {
    #[default]
    #[cfg_attr(rust_js, rust_js::name = "a")]
    A,
    #[cfg_attr(rust_js, rust_js::name = "b")]
    B { n: u32 },
}

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Slot {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<Plain>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Slot {
    pub fn new(state: Plain) -> Self {
        Slot {
            state: RefCell::new(state),
        }
    }
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

thread_local! {
    static CURRENT: RefCell<Option<Rc<Task>>> = const { RefCell::new(None) };
}

impl Default for Status {
    // Reads the task's state, then writes it: the take moves out what this
    // wrote, as it runs before the take borrows the cell.
    fn default() -> Self {
        CURRENT.with_borrow(|current| {
            if let Some(task) = current {
                println!("default sees {:?}", *task.state.borrow());
                if let Status::Pending { tries } = &mut *task.state.borrow_mut() {
                    *tries += 10;
                }
            }
        });
        Status::Pending { tries: 0 }
    }
}

fn main() {
    let slot = Slot::new(Plain::B { n: 1 });
    let old = slot.state.take();
    println!("{} {:?} {:?}", matches!(old, Plain::B { .. }), old, slot.state.borrow());

    let task = Rc::new(Task::new(Status::Pending { tries: 1 }, "task"));
    CURRENT.with_borrow_mut(|current| *current = Some(task.clone()));
    let old = task.state.take();
    println!("{} took {:?}, left {:?}", task.name, old, task.state.borrow());
    *task.state.borrow_mut() = Status::Done {
        value: String::from("x"),
    };
    let old = task.state.take();
    println!("{} took {:?}, left {:?}", task.name, old, task.state.borrow());
}
