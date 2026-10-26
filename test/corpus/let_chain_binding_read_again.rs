// A let chain's binding of a field its test read is the field read again
// where the body reads it first (ADR 0866, amended); where the body's code
// may change the field before it's read, or it's read twice, the binding
// stays the value the test read.

use std::cell::Cell;

struct Task {
    runs: Cell<u32>,
}

impl Task {
    fn run(&self, owner: &Fiber, clear: bool) -> u32 {
        self.runs.set(self.runs.get() + 1);
        if clear {
            owner.task.set(None);
        }
        self.runs.get()
    }
}

struct Fiber {
    task: Cell<Option<&'static Task>>,
}

fn once(fiber: Option<&Fiber>) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.task.get()
    {
        return task.run(fiber, false);
    }
    0
}

fn twice(fiber: Option<&Fiber>) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.task.get()
    {
        return task.run(fiber, true) + task.run(fiber, false);
    }
    0
}

fn cleared_first(fiber: Option<&Fiber>) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.task.get()
    {
        fiber.task.set(None);
        return task.run(fiber, false) * 10;
    }
    0
}

fn main() {
    let task: &'static Task = Box::leak(Box::new(Task { runs: Cell::new(0) }));
    let fiber = Fiber { task: Cell::new(Some(task)) };
    println!("{} {} {}", once(Some(&fiber)), once(Some(&fiber)), once(None));
    println!("{} {}", twice(Some(&fiber)), fiber.task.get().is_none());
    fiber.task.set(Some(task));
    println!("{} {} {}", cleared_first(Some(&fiber)), fiber.task.get().is_none(), once(Some(&fiber)));
}
