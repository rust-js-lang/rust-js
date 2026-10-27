// A thread-local `Vec` taken, or replaced, where that's the first its
// statement does, is its module's variable given the new one, `const old =
// pending; pending = [];`, as React's flushPendingUnsafeLifecycleWarnings
// sets `pending = []`, not emptied in place (ADR 0270, amended): what's
// pushed while the old one is walked goes in the new one, as in Rust.
use std::cell::RefCell;

thread_local! {
    static PENDING: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn record(id: u32) {
    PENDING.with_borrow_mut(|pending| pending.push(id));
}

fn flush() -> Vec<u32> {
    let mut seen = Vec::new();
    if PENDING.with_borrow(|pending| !pending.is_empty()) {
        for id in PENDING.take() {
            if id < 3 {
                record(id + 10);
            }
            seen.push(id);
        }
    }
    seen
}

fn swapped() -> Vec<u32> {
    let before = PENDING.replace(vec![7, 8]);
    record(9);
    before
}

fn dropped() -> usize {
    PENDING.take();
    PENDING.with_borrow(Vec::len)
}

fn main() {
    record(1);
    record(5);
    println!("{:?} {:?}", flush(), PENDING.with_borrow(Clone::clone));
    println!("{:?} {:?}", swapped(), PENDING.with_borrow(Clone::clone));
    println!("{} {:?}", dropped(), flush());
}
