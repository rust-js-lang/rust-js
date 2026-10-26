// A let chain's binding of a field, read before a later test of the chain
// that runs code, which may change the field: the binding holds what was
// read before it, so it stays before the test, not after it in the body
// (ADR 0842, amended). Where nothing after it in the test runs code, the
// chain is one test, its binding in its body.

use std::cell::Cell;

struct Fiber {
    task: Cell<Option<&'static str>>,
}

fn probe(fiber: &Fiber) -> bool {
    fiber.task.set(None);
    true
}

fn probed(fiber: Option<&Fiber>) -> usize {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.task.get()
        && probe(fiber)
    {
        return task.len();
    }
    4
}

fn kept(fiber: Option<&Fiber>, min: usize) -> usize {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.task.get()
        && task.len() > min
    {
        fiber.task.set(None);
        return task.len();
    }
    5
}

fn main() {
    let fiber = Fiber { task: Cell::new(Some("render")) };
    println!("{} {:?}", probed(Some(&fiber)), fiber.task.get());
    println!("{} {}", probed(Some(&fiber)), probed(None));
    fiber.task.set(Some("commit"));
    println!("{} {:?} {}", kept(Some(&fiber), 2), fiber.task.get(), kept(Some(&fiber), 2));
}
