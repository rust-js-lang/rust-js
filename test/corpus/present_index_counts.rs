// An index an item is there at counts on without its wrap (ADR 0881,
// amended): `while let Some(x) = v.get(i)`, of a thread-local's
// `with_borrow` too, as React's ReactPostPaintCallback flushes its
// callbacks, and past a `let Some(x) = v.get(i) else { break };`; where
// it isn't there, `i + 1` still wraps.

use std::cell::RefCell;

thread_local! {
    static CALLBACKS: RefCell<Vec<&'static dyn Fn(f64)>> = const { RefCell::new(Vec::new()) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn first(end_time: f64) {
    LOG.with_borrow_mut(|log| log.push(format!("first {end_time}")));
}

fn second(end_time: f64) {
    LOG.with_borrow_mut(|log| log.push(format!("second {end_time}")));
    // One scheduled while they run runs too.
    CALLBACKS.with_borrow_mut(|callbacks| {
        if callbacks.len() < 3 {
            callbacks.push(&first);
        }
    });
}

fn flush(end_time: f64) -> usize {
    let mut i = 0;
    while let Some(callback) = CALLBACKS.with_borrow(|callbacks| callbacks.get(i).copied()) {
        callback(end_time);
        i += 1;
    }
    CALLBACKS.set(Vec::new());
    i
}

fn summed(items: &[u64]) -> u64 {
    let mut total = 0;
    let mut i = 0;
    while let Some(&item) = items.get(i) {
        total += item;
        i += 1;
    }
    total
}

fn until_zero(counts: &[u32]) -> u32 {
    let mut i = 0;
    let mut total = 0;
    loop {
        let Some(&n) = counts.get(i) else { break };
        if n == 0 {
            break;
        }
        total += n;
        i += 1;
    }
    total
}

fn skipped(items: &[u32], i: usize) -> usize {
    if let Some(&item) = items.get(i) {
        item as usize
    } else {
        i.wrapping_add(1)
    }
}

fn main() {
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.push(&first);
        callbacks.push(&second);
    });
    let flushed = flush(16.5);
    LOG.with_borrow(|log| println!("{flushed} {}", log.join(", ")));
    println!("{:?} {:?}", summed(&[1, 2, 3, u64::MAX - 6]), summed(&[]));
    println!("{} {}", until_zero(&[3, 4, 0, 5]), until_zero(&[1, 2]));
    println!("{} {} {}", skipped(&[7], 0), skipped(&[7], 5), skipped(&[7], usize::MAX));
}
