// A `&mut` to a place that stores what's written its own way, made a `&mut
// dyn Trait`: its pair stores so too, as its handle does (#977). A
// `#[rust_js::nullable]` field's `None` is `null` (ADR 0772), and a class's
// flattened state written whole sets its keys on the class's object (ADR
// 1044); a pair that assigned the place as it's given left `undefined` in
// the field, and replaced the variable that holds the object.
use std::cell::RefCell;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
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

struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    owner: Option<u32>,
}

trait Set<T> {
    fn set(&mut self, value: T);
}

impl<T> Set<T> for T {
    fn set(&mut self, value: T) {
        *self = value;
    }
}

fn cleared(slot: &mut dyn Set<Option<u32>>) {
    slot.set(None);
}

fn finished(state: &mut dyn Set<Status>, value: &str) {
    state.set(Status::Done {
        value: value.to_string(),
    });
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn absent(x: Option<u32>) -> bool {
    x.is_none()
}

fn main() {
    let mut fiber = Fiber { owner: Some(3) };
    cleared(&mut fiber.owner);
    println!("{} {:?}", absent(fiber.owner), fiber.owner);

    let mut task = Task::new(Status::Pending { tries: 1 }, "task");
    finished(task.state.get_mut(), "b");
    println!("{} {:?}", task.name, task.state.borrow());
}
