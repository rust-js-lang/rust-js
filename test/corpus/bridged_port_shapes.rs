// React's port's shapes whose wraps THIR's ranges leave out and MIR's
// intervals don't prove yet, which the bridge leaves out (ADR 1137,
// test/intervals-bridged.txt): a `Vec` behind a `RefCell` tested not
// empty, then indexed at `len() - 1` through borrows of its own, as
// ReactNoopServer does (#1741); a counter a `&mut` capture holds, below
// a queue's length, in `catch_unwind`'s closure, as ReactAct flushes its
// queue (#1701); and a thread-local's `with_borrow` whose `get(i).cloned()`
// is `Some`, as ReactDOMEventReplaying replays (#1701).

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static TARGETS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn push_child(stack: &RefCell<Vec<u32>>, roots: &mut Vec<u32>, instance: u32) {
    if stack.borrow().is_empty() {
        roots.push(instance);
    } else {
        let parent = stack.borrow()[stack.borrow().len() - 1];
        roots.push(parent * 100 + instance);
    }
    stack.borrow_mut().push(instance);
}

fn flush(queue: &RefCell<Vec<u32>>) -> u32 {
    let mut i = 0;
    let mut total = 0;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        while i < queue.borrow().len() {
            let item = queue.borrow()[i];
            if item == 0 {
                break;
            }
            total += item;
            i += 1;
        }
    }));
    assert!(caught.is_ok());
    queue.borrow_mut().drain(0..i);
    total
}

fn replay() -> Vec<String> {
    let mut seen = Vec::new();
    let mut i = 0;
    while let Some(target) = TARGETS.with_borrow(|targets| targets.get(i).cloned()) {
        seen.push(format!("{i}:{target}"));
        i += 1;
    }
    seen
}

fn main() {
    let stack = RefCell::new(Vec::new());
    let mut roots = Vec::new();
    for instance in [1, 2, 3] {
        push_child(&stack, &mut roots, instance);
    }
    println!("{roots:?}");

    let queue = RefCell::new(vec![4, 5, 0, 6]);
    println!("{} {:?}", flush(&queue), queue.borrow());

    TARGETS.with_borrow_mut(|targets| targets.extend(["a".to_string(), "b".to_string()]));
    println!("{:?}", replay());
}
