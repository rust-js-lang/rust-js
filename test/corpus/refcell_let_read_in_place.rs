// A `let` that ends a `RefCell`'s borrow, stored by the statement right
// after it, React's createWorkInProgress's `let child =
// current.child.borrow().clone(); work_in_progress.child.replace(child);`:
// where nothing that statement runs before the read does anything, and its
// name is the field's own, the place is read there, `workInProgress.child =
// current.child;` (ADR 0866, amended). Not where something runs first,
// which may change the cell, nor where it's read again, nor where it's an
// argument, which the program names, nor one named otherwise, nor a `let`
// of a value that isn't a borrow's clone, a `Cell`'s `get()`.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fiber {
    name: String,
    index: Cell<u32>,
    // Stored as `null` where it's `None`, `child ?? null` (ADR 0586).
    #[cfg_attr(rust_js, rust_js::nullable)]
    child: RefCell<Option<Rc<Fiber>>>,
    sibling: RefCell<Option<Rc<Fiber>>>,
}

fn fiber(name: &str) -> Rc<Fiber> {
    Rc::new(Fiber {
        name: name.to_string(),
        index: Cell::new(name.len() as u32),
        child: RefCell::new(None),
        sibling: RefCell::new(None),
    })
}

fn name_of(f: &Option<Rc<Fiber>>) -> String {
    f.as_ref().map_or("none".to_string(), |f| f.name.clone())
}

/// A store: `workInProgress.child = current.child;`.
fn copy_child(work_in_progress: &Fiber, current: &Fiber) {
    let child = current.child.borrow().clone();
    work_in_progress.child.replace(child);
}

/// An argument: the program's name, as upstream's `const handlerOnStack =
/// ..; push(.., handlerOnStack, ..)` keeps it.
fn report(node: &Fiber, log: &mut Vec<String>) {
    let sibling = node.sibling.borrow().clone();
    log.push(format!("{} {}", node.name, name_of(&sibling)));
}

/// A call that runs first may change the cell: read before it, as Rust
/// reads it.
fn emptied(f: &Fiber) -> Option<Rc<Fiber>> {
    f.child.replace(None)
}

fn read_first(work_in_progress: &Fiber, current: &Fiber) -> String {
    let child = current.child.borrow().clone();
    let old = emptied(current);
    work_in_progress.child.replace(child);
    let now = work_in_progress.child.borrow().clone();
    format!("{} {}", name_of(&old), name_of(&now))
}

/// Read in a call whose earlier argument runs code: read before it.
fn later_argument(current: &Fiber) -> String {
    let child = current.child.borrow().clone();
    pair(emptied(current), child)
}

fn pair(a: Option<Rc<Fiber>>, b: Option<Rc<Fiber>>) -> String {
    format!("{} {}", name_of(&a), name_of(&b))
}

/// A `let` of a `Cell`'s value, no borrow's: the program's name kept.
fn copy_index(work_in_progress: &Fiber, current: &Fiber) {
    let index = current.index.get();
    work_in_progress.index.set(index);
}

/// Named otherwise than the field it reads: the program's name, as
/// React's updateReducerImpl's `const pendingFirst = pendingQueue.next;
/// baseQueue.next = pendingFirst;`.
fn swap_children(base: &Fiber, pending: &Fiber) {
    let base_first = base.child.borrow().clone();
    let pending_first = pending.child.borrow().clone();
    base.child.replace(pending_first);
    pending.child.replace(base_first);
}

/// Read twice: its name kept.
fn twice(work_in_progress: &Fiber, current: &Fiber) -> String {
    let sibling = current.sibling.borrow().clone();
    work_in_progress.sibling.replace(sibling.clone());
    name_of(&sibling)
}

fn main() {
    let current = fiber("current");
    current.child.replace(Some(fiber("a")));
    current.sibling.replace(Some(fiber("b")));
    let work = fiber("work");
    copy_child(&work, &current);
    let copied = work.child.borrow().clone();
    println!("copied {}", name_of(&copied));
    let mut log = Vec::new();
    report(&current, &mut log);
    report(&work, &mut log);
    println!("{log:?}");
    println!("read first {}", read_first(&fiber("w"), &current));
    current.child.replace(Some(fiber("c")));
    println!("later argument {}", later_argument(&current));
    swap_children(&work, &current);
    let swapped = (work.child.borrow().clone(), current.child.borrow().clone());
    println!("swapped {} {}", name_of(&swapped.0), name_of(&swapped.1));
    let first = twice(&work, &current);
    let sibling = work.sibling.borrow().clone();
    println!("twice {first} {}", name_of(&sibling));
    copy_index(&work, &current);
    println!("index {}", work.index.get());
}
