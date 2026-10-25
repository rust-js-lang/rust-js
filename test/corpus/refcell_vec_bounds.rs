// A `Vec` a `RefCell` holds, by the cell (ADR 0881): an index its loop's
// test reads below the length, with nothing between, `queue[i]`, as React's
// ReactAct reads its queue; an item set, `queue[i] = x`, then a drain to
// it, `queue.splice(0, i)`. Anything that may change the cell meanwhile, a
// call, a drop of what runs code, a `&mut` through its guard, and the
// index is checked again, as Rust's panics say.
use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

/// Empties its queue where it's dropped.
struct Clearer(Rc<RefCell<Vec<u32>>>);

impl Drop for Clearer {
    fn drop(&mut self) {
        self.0.borrow_mut().clear();
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn sum(queue: &RefCell<Vec<u32>>) -> u32 {
    let mut i = 0;
    let mut total = 0;
    while i < queue.borrow().len() {
        total += queue.borrow()[i];
        i += 1;
    }
    total
}

/// Sets the item it stops at, and drains what's before it.
fn settle(queue: &RefCell<Vec<u32>>, stop: u32) -> usize {
    let mut i = 0;
    while i < queue.borrow().len() {
        let mut item = queue.borrow()[i];
        if item == stop {
            item += 100;
            queue.borrow_mut()[i] = item;
            queue.borrow_mut().drain(0..i);
            return i;
        }
        i += 1;
    }
    i
}

/// Sets a task, whose replaced one's drop runs while the queue's guard
/// holds it, then drains to it.
fn settle_task(queue: &RefCell<Vec<Rc<dyn Fn() -> u32>>>, at: usize, task: Rc<dyn Fn() -> u32>) -> usize {
    if at < queue.borrow().len() {
        queue.borrow_mut()[at] = task;
        queue.borrow_mut().drain(0..at);
    }
    queue.borrow().len()
}

/// Sets an item after a call, checked, which shows the drain to it in
/// bounds, as React's ReactAct sets its queue's after a task runs.
fn settle_after(queue: &RefCell<Vec<u32>>, i: usize, touch: &dyn Fn()) -> usize {
    touch();
    queue.borrow_mut()[i] = 0;
    queue.borrow_mut().drain(0..i);
    queue.borrow().len()
}

fn called_between(queue: &RefCell<Vec<u32>>, clear: &dyn Fn()) -> u32 {
    let mut i = 0;
    let mut total = 0;
    while i < queue.borrow().len() {
        clear();
        total += queue.borrow()[i];
        i += 1;
    }
    total
}

fn dropped_between(queue: &RefCell<Vec<u32>>, clearer: Clearer) -> u32 {
    if !queue.borrow().is_empty() {
        {
            let _gone = clearer;
        }
        return queue.borrow()[0];
    }
    0
}

fn replaced_between(queue: &RefCell<Vec<u32>>, slots: &RefCell<Vec<Clearer>>, other: Clearer) -> u32 {
    if !queue.borrow().is_empty() {
        slots.borrow_mut()[0] = other;
        return queue.borrow()[0];
    }
    0
}

fn cleared_by_its_guard(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        let all = &mut *items;
        all.clear();
        return items[i];
    }
    0
}

fn tested_before(queue: &RefCell<Vec<u32>>, clear: &dyn Fn()) -> u32 {
    let there = !queue.borrow().is_empty();
    clear();
    if there { queue.borrow()[0] } else { 0 }
}

fn main() {
    let queue = Rc::new(RefCell::new(vec![1, 2, 3, 4]));
    println!("sum: {}", sum(&queue));
    println!("settle: {}", settle(&queue, 3));
    println!("queue: {:?}", queue.borrow());
    println!("settle none: {}", settle(&queue, 9));
    let tasks: RefCell<Vec<Rc<dyn Fn() -> u32>>> = RefCell::new(vec![Rc::new(|| 1), Rc::new(|| 2), Rc::new(|| 3)]);
    println!("settle task: {}", settle_task(&tasks, 1, Rc::new(|| 20)));
    println!("tasks: {:?}", tasks.borrow().iter().map(|task| task()).collect::<Vec<_>>());
    let after = RefCell::new(vec![1, 2, 3]);
    println!("settle after: {}", settle_after(&after, 1, &|| println!("touched")));
    check("settle after none", || settle_after(&after, 5, &|| {}));

    let cleared = Rc::clone(&queue);
    check("called between", || {
        called_between(&queue, &|| cleared.borrow_mut().clear())
    });
    queue.borrow_mut().extend([5, 6]);
    check("dropped between", || {
        dropped_between(&queue, Clearer(Rc::clone(&queue)))
    });
    queue.borrow_mut().extend([7, 8]);
    let slots = RefCell::new(vec![Clearer(Rc::clone(&queue))]);
    check("replaced between", || {
        replaced_between(&queue, &slots, Clearer(Rc::new(RefCell::new(Vec::new()))))
    });
    queue.borrow_mut().extend([9]);
    check("cleared by its guard", || cleared_by_its_guard(&queue, 0));
    queue.borrow_mut().extend([10]);
    let cleared = Rc::clone(&queue);
    check("tested before", || {
        tested_before(&queue, &|| cleared.borrow_mut().clear())
    });
}
