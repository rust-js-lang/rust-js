// `is_sorted_by` of an iterator whose items need no drop, whose closure
// holds a value with a destructor: std's `all` drops the closure, then
// `is_sorted_by` drops the iterator, so the closure's value is dropped
// before the iterator closes, where two are out of order, at the end, and
// as the closure or a step panics (#2029).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counter {
    n: u32,
    end: u32,
    panics_at: Option<u32>,
}

impl Counter {
    fn new(end: u32) -> Self {
        Counter { n: 0, end, panics_at: None }
    }
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.panics_at == Some(self.n) {
            panic!("step {}", self.n);
        }
        (self.n < self.end).then(|| {
            self.n += 1;
            self.n
        })
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop counter at {}", self.n);
    }
}

fn main() {
    let guard = Guard("out of order");
    let sorted = Counter::new(3).take(2).is_sorted_by(move |a, b| {
        let _ = &guard;
        a >= b
    });
    println!("sorted {sorted}");

    let guard = Guard("at its end");
    let sorted = Counter::new(3).is_sorted_by(move |a, b| {
        let _ = &guard;
        a <= b
    });
    println!("sorted {sorted}");

    let guard = Guard("at a take's limit");
    let sorted = Counter::new(5).take(3).is_sorted_by(move |a, b| {
        let _ = &guard;
        a <= b
    });
    println!("sorted {sorted}");

    let guard = Guard("by a reference");
    let mut counter = Counter::new(4);
    let sorted = counter.by_ref().is_sorted_by(move |a, b| {
        let _ = &guard;
        a > b
    });
    println!("sorted {sorted} then {:?}", counter.next());
    drop(counter);

    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let guard = Guard("f panics");
        Counter::new(4).is_sorted_by(move |a, b| {
            let _ = &guard;
            if *b == 3 {
                panic!("f at {b}");
            }
            a <= b
        })
    }));
    println!("unwound {}", unwound.is_err());

    let unwound = catch_unwind(AssertUnwindSafe(|| {
        let guard = Guard("a step panics");
        let counter = Counter { n: 0, end: 4, panics_at: Some(2) };
        counter.is_sorted_by(move |a, b| {
            let _ = &guard;
            a <= b
        })
    }));
    println!("unwound {}", unwound.is_err());
}
