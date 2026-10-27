// Two places set the same variable or constant one after the other are two
// statements, as written: ADR 0968 made them one, `a = b = v`, and the user
// reverted it (2026-10-27), as upstream React writes both. Each still sets
// what Rust sets, in Rust's order, `holder.current = node;
// holder.current.next = node;` too.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Hook {
    pub memoized_state: Cell<u32>,
    pub base_state: Cell<u32>,
    pub next: RefCell<Option<Rc<Hook>>>,
}

pub struct Holder {
    pub current: RefCell<Option<Rc<Hook>>>,
}

thread_local! {
    static WORK_IN_PROGRESS_HOOK: RefCell<Option<Rc<Hook>>> = const { RefCell::new(None) };
}

fn hook(state: u32) -> Rc<Hook> {
    Rc::new(Hook {
        memoized_state: Cell::new(state),
        base_state: Cell::new(state),
        next: RefCell::new(None),
    })
}

fn mount_state(hook: &Hook, initial_state: u32) {
    hook.base_state.set(initial_state);
    hook.memoized_state.set(initial_state);
}

fn reset(hook: &Hook) {
    hook.base_state.set(0);
    hook.memoized_state.set(0);
}

fn set_both(hook: &Hook, base: u32, memoized: u32) {
    hook.base_state.set(base);
    hook.memoized_state.set(memoized);
}

/// Three or more set alike stay a statement each.
fn cleared(a: &Hook, b: &Hook, c: &Hook) {
    a.base_state.set(0);
    b.base_state.set(0);
    c.base_state.set(0);
}

fn bumped(hook: &Hook, n: u32) {
    hook.base_state.set(n + 1);
    hook.memoized_state.set(n + 1);
}

fn append(fiber: &Holder, hook: Rc<Hook>) {
    let last = WORK_IN_PROGRESS_HOOK.with_borrow(|last| last.clone());
    match last {
        None => {
            fiber.current.replace(Some(hook.clone()));
            WORK_IN_PROGRESS_HOOK.set(Some(hook));
        }
        Some(last) => {
            last.next.replace(Some(hook.clone()));
            WORK_IN_PROGRESS_HOOK.set(Some(hook));
        }
    }
}

pub struct Pinned {
    pub current: RefCell<Rc<Hook>>,
}

fn current_of(pinned: &Pinned, node: Rc<Hook>) {
    *pinned.current.borrow_mut() = node.clone();
    pinned.current.borrow().next.replace(Some(node));
}

fn moved_to(first: Rc<Hook>, node: Rc<Hook>) -> Rc<Hook> {
    let mut current = first;
    current.next.replace(None);
    current = node.clone();
    current.next.replace(Some(node));
    current
}

fn main() {
    let first = hook(1);
    mount_state(&first, 5);
    println!("{} {}", first.memoized_state.get(), first.base_state.get());
    bumped(&first, 7);
    println!("{} {}", first.memoized_state.get(), first.base_state.get());
    set_both(&first, 8, 9);
    println!("{} {}", first.memoized_state.get(), first.base_state.get());
    reset(&first);
    cleared(&first, &first, &first);
    println!("{} {}", first.memoized_state.get(), first.base_state.get());
    let fiber = Holder {
        current: RefCell::new(None),
    };
    append(&fiber, first.clone());
    let second = hook(2);
    append(&fiber, second.clone());
    let next = first.next.borrow().as_ref().map(|next| next.memoized_state.get());
    let last = WORK_IN_PROGRESS_HOOK.with_borrow(|last| last.as_ref().map(|last| last.memoized_state.get()));
    println!("{next:?} {last:?}");
    let node = hook(3);
    let pinned = Pinned {
        current: RefCell::new(hook(4)),
    };
    current_of(&pinned, node.clone());
    println!("{}", Rc::ptr_eq(node.next.borrow().as_ref().unwrap(), &node));
    node.next.replace(None);
    let other = hook(5);
    let moved = moved_to(other.clone(), node.clone());
    println!("{} {}", Rc::ptr_eq(&moved, &node), other.next.borrow().is_none());
    node.next.replace(None);
}
