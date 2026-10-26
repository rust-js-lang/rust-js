// A thread-local `RefCell`'s own `take` and `replace` (ADR 0595,
// amended): its cell's, borrowed mutably to do it, as React's reconciler
// takes its queue, `QUEUE.take()`; and a `Cell`'s, read and written.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    static TICKS: Cell<u32> = const { Cell::new(1) };
    static QUEUE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static COUNT: RefCell<u32> = const { RefCell::new(0) };
    static NAME: RefCell<String> = RefCell::new(String::from("first"));
    static CURRENT: RefCell<Option<Rc<String>>> = const { RefCell::new(None) };
}

fn push(x: u32) {
    QUEUE.with_borrow_mut(|queue| queue.push(x));
    COUNT.with_borrow_mut(|count| *count += 1);
}

fn finish() -> u32 {
    let queue = QUEUE.take();
    queue.into_iter().sum()
}

fn main() {
    push(1);
    push(2);
    println!("{} {:?}", finish(), QUEUE.take());
    push(3);
    // Taken while another is borrowed, which it can't be.
    LOG.with_borrow_mut(|log| log.extend(QUEUE.take()));
    push(3);
    let old = QUEUE.replace(vec![7, 8]);
    println!("{old:?} {:?}", QUEUE.with_borrow(|queue| queue.clone()));
    println!("{} {}", COUNT.replace(10), COUNT.take());
    println!("{}", COUNT.with_borrow(|count| *count));
    println!("{} {:?}", NAME.replace(String::from("second")), NAME.take());
    let current = Rc::new(String::from("fiber"));
    println!("{:?}", CURRENT.replace(Some(Rc::clone(&current))));
    let taken = CURRENT.take();
    let same = taken.as_ref().is_some_and(|taken| Rc::ptr_eq(taken, &current));
    println!("{taken:?} {same}");
    println!("{:?}", CURRENT.take());
    println!("{:?}", LOG.take());
    println!("{} {} {}", TICKS.replace(5), TICKS.take(), TICKS.get());
}
