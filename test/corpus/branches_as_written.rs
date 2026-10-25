// What React's `act` writes, as it writes it (ADR 0879): an `if !c` with an
// `else`, its branches in their order, which MIR's `switchInt` turns; an
// `if let .. else { break }` in a loop, its `else` kept; a value written
// `if c { x } else { false }`, a conditional, not `c && x`; a call's value
// given to a variable a closure captured, `result = callback();`, with no
// temporary, where a place JS reaches first, `a[at.get()]`, which the
// call may change, keeps it; and a clone of what a `move` closure captured, read by
// its name, but not of what one only borrowed, given another value after.

use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

thread_local! {
    static DID_USE_PROMISE: Cell<bool> = const { Cell::new(false) };
    static ACT_QUEUE: RefCell<Option<Rc<RefCell<Vec<u32>>>>> = const { RefCell::new(None) };
}

const DISABLE_LEGACY_MODE: bool = false;

fn flush_act_queue(queue: &RefCell<Vec<u32>>, callback: &dyn Fn(u32) -> Option<u32>) -> usize {
    let mut i = 0;
    while i < queue.borrow().len() {
        let mut task = queue.borrow()[i];
        loop {
            DID_USE_PROMISE.set(false);
            let continuation = callback(task);
            if let Some(continuation) = continuation {
                if DID_USE_PROMISE.get() {
                    queue.borrow_mut()[i] = task;
                    return i;
                }
                task = continuation;
            } else {
                break;
            }
        }
        i += 1;
    }
    queue.borrow_mut().clear();
    i
}

fn recursively_flush(log: &mut Vec<String>) {
    let queue = ACT_QUEUE.with(|q| q.borrow().clone());
    if let Some(queue) = queue {
        if !queue.borrow().is_empty() {
            let flushed = flush_act_queue(&queue, &|task| if task < 4 { Some(task * 3) } else { None });
            log.push(format!("flushed {flushed}"));
        } else {
            ACT_QUEUE.with(|q| q.replace(None));
            log.push("empty".to_string());
        }
    }
}

fn batching(legacy: bool) -> bool {
    if !DISABLE_LEGACY_MODE { legacy } else { false }
}

fn either(fast: bool, slow: &dyn Fn() -> bool) -> bool {
    let chosen = if fast { true } else { slow() };
    chosen
}

fn act(callback: &dyn Fn() -> Option<u32>) -> Option<u32> {
    let mut result = None;
    if panic::catch_unwind(AssertUnwindSafe(|| {
        result = callback();
    }))
    .is_err()
    {
        println!("caught");
    }
    result
}

fn bump(at: &Cell<usize>) -> u32 {
    at.set(at.get() + 1);
    7
}

fn placed() -> [u32; 3] {
    let mut a = [0; 3];
    let at = Cell::new(0);
    a[at.get()] = bump(&at);
    a
}

type Settle = Box<dyn Fn(u32) -> usize>;

fn thenable(prev: Option<Rc<Vec<u32>>>) -> Box<dyn Fn() -> (Settle, Settle)> {
    let queue = prev.clone().unwrap_or_else(|| Rc::new(Vec::new()));
    Box::new(move || {
        let prev = prev.clone();
        let queue = Rc::clone(&queue);
        let rejected_prev = prev.clone();
        (
            Box::new(move |x| queue.len() + x as usize + prev.as_ref().map_or(0, |p| p.len())),
            Box::new(move |x| x as usize + rejected_prev.as_ref().map_or(0, |p| p.len())),
        )
    })
}

fn borrowed_then_given_another() -> i32 {
    let mut a = Rc::new(1);
    let make = || {
        let b = a.clone();
        move || *b
    };
    let g = make();
    a = Rc::new(2);
    *a * 10 + g()
}

fn main() {
    let mut log = Vec::new();
    ACT_QUEUE.with(|q| q.replace(Some(Rc::new(RefCell::new(vec![1, 2, 5])))));
    recursively_flush(&mut log);
    recursively_flush(&mut log);
    recursively_flush(&mut log);
    println!("{log:?}");

    let queue = RefCell::new(vec![1, 7]);
    let suspended = flush_act_queue(&queue, &|task| {
        if task > 6 {
            DID_USE_PROMISE.set(true);
        }
        if task < 10 { Some(task + 1) } else { None }
    });
    println!("{suspended} {:?}", queue.borrow());

    println!("{} {}", batching(true), batching(false));
    println!("{} {}", either(true, &|| false), either(false, &|| true));
    println!("{:?}", act(&|| Some(3)));
    println!("{:?}", act(&|| panic!("thrown")));
    println!("{:?}", placed());

    let make = thenable(Some(Rc::new(vec![1, 2])));
    let (resolve, reject) = make();
    println!("{} {}", resolve(3), reject(4));
    let (resolve, reject) = thenable(None)();
    println!("{} {}", resolve(3), reject(4));
    println!("{}", borrowed_then_given_another());
}
