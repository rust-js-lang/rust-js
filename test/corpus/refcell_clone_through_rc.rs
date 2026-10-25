// What std's `Clone` of a type runs is what its impl's bounds say (ADR
// 0868): `Option<Rc<..>>`'s clones no task, so a cell borrowed for the
// clone meets no other, and a derived `Clone` of a task reads its fields,
// a function, which runs nothing. So neither cell is counted, as React's
// `actQueue` of `RendererTask`s isn't; a transparent struct of a function
// written through a `&mut` is its function, which a clone is too (ADR
// 0584, amended). A derived `Clone` whose field's own `Clone` borrows the
// cell still meets it, and panics as Rust does.
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

#[derive(Clone)]
#[repr(transparent)]
struct Task(fn(u32) -> Option<Task>);

fn last(_: u32) -> Option<Task> {
    None
}

fn first(n: u32) -> Option<Task> {
    (n > 1).then_some(Task(last))
}

struct Internals {
    #[cfg_attr(rust_js, rust_js::nullable)]
    queue: RefCell<Option<Rc<RefCell<Vec<Task>>>>>,
}

fn flush(queue: &RefCell<Vec<Task>>) -> u32 {
    let mut i = 0;
    let mut ran = 0;
    while i < queue.borrow().len() {
        let mut task = queue.borrow()[i].clone();
        while let Some(next) = (task.0)(i as u32) {
            ran += 1;
            if i == 3 {
                queue.borrow_mut()[i] = next;
                return ran;
            }
            task = next;
        }
        i += 1;
    }
    queue.borrow_mut().clear();
    ran
}

thread_local! {
    static WHO: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static OTHER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

struct Asks;

impl Clone for Asks {
    fn clone(&self) -> Self {
        WHO.with_borrow_mut(|who| who.push(1));
        Asks
    }
}

struct Other;

impl Clone for Other {
    fn clone(&self) -> Self {
        OTHER.with_borrow_mut(|other| other.push(1));
        Other
    }
}

#[derive(Clone)]
struct Holds {
    _asks: Asks,
}

fn main() {
    let internals = Internals {
        queue: RefCell::new(None),
    };
    let queue = internals.queue.borrow().clone();
    let queue = queue.unwrap_or_else(|| Rc::new(RefCell::new(vec![Task(first); 5])));
    internals.queue.replace(Some(Rc::clone(&queue)));
    let ran = flush(&queue);
    println!("{ran} {}", queue.borrow().len());
    let held = WHO.with_borrow(|_| panic::catch_unwind(AssertUnwindSafe(|| Holds { _asks: Asks }.clone())).is_err());
    let bounded = OTHER.with_borrow(|_| panic::catch_unwind(AssertUnwindSafe(|| Some(Some(Other)).clone())).is_err());
    println!("{held} {bounded}");
}
