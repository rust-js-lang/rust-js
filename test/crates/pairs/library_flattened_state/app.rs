//! What's moved out keeps its own keys as the task's change.
use dep::{Status, Task};
use std::mem;

pub fn main() {
    let mut task = Task::new(Status::Pending { tries: 3 }, "task");
    let mut old = mem::replace(task.state.get_mut(), Status::Halted);
    if let Status::Pending { tries } = &mut old {
        *tries += 10;
    }
    println!("{old:?}, {task}");
    let taken = task.state.replace(Status::Done {
        value: String::from("a"),
    });
    *task.state.borrow_mut() = Status::Pending { tries: 7 };
    println!("{taken:?}, {task}");
}
