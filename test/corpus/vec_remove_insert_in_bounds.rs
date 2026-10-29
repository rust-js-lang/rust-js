// A `Vec`'s `remove(i)` and `insert(i, x)` whose index the body's ranges
// show in bounds, below the length to remove, at most it to insert, are
// JS's own: React's ReactDOMEventReplaying drains its targets,
// `targetContainers.shift()` while it isn't empty, and puts one in place,
// `splice(i, 0, target)` (rd103). One not shown in bounds is checked, and
// panics as Rust's does.
use std::panic::{AssertUnwindSafe, catch_unwind};

fn drained(mut targets: Vec<u32>) -> u32 {
    let mut sum = 0;
    while !targets.is_empty() {
        let first = targets.remove(0);
        sum = sum * 10 + first;
    }
    sum
}

fn at_end(targets: &mut Vec<u32>) {
    let n = targets.len();
    targets.insert(n, 9);
}

fn second(targets: &mut Vec<u32>) -> u32 {
    if targets.len() > 2 {
        targets.insert(1, 5);
        return targets.remove(2);
    }
    0
}

// What it removed, read and not.
fn taken(targets: &mut Vec<u32>) {
    if targets.len() > 2 {
        targets.remove(2);
    }
}

fn picked(targets: &mut Vec<u32>) -> u32 {
    if targets.len() > 2 {
        return targets.remove(1);
    }
    0
}

// Past what its test shows of the length: checked, and it panics.
fn past_least(targets: &mut Vec<u32>) {
    if targets.len() > 2 {
        targets.insert(5, 1);
    }
}

fn unknown(targets: &mut Vec<u32>, i: usize) -> u32 {
    targets.insert(i, 7);
    targets.remove(i)
}

fn main() {
    println!("{}", drained(vec![1, 2, 3]));
    let mut targets = vec![4, 6, 8];
    at_end(&mut targets);
    println!("{targets:?}");
    println!("{} {targets:?}", second(&mut targets));
    println!("{} {targets:?}", unknown(&mut targets, 4));
    let past = catch_unwind(AssertUnwindSafe(|| unknown(&mut targets, 9))).is_err();
    println!("{past} {targets:?}");
    let mut four = vec![1, 2, 3, 4];
    taken(&mut four);
    println!("{} {four:?}", picked(&mut four) + 10);
    let mut three = vec![1, 2, 3];
    let past = catch_unwind(AssertUnwindSafe(|| past_least(&mut three))).is_err();
    println!("{past} {three:?}");
}
