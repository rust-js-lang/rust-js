// A length held before a `push`, below the length past it: `v[n]` of `n =
// v.len()` read with no check, what was at most the length, and the heap's
// `sift_up` given it as its index; a held length tested equal to the length
// again, `properties[prev_length - 1]` (ADR 1168, #1948). What only looks
// alike keeps its check and panics as Rust's does: `n - 1` of an empty
// vector, which wraps; a length past a `pop`, a `clear()` or a new vector;
// another vector's push, and an index below another slice's length; a held
// length one more by a wrapping add; a length tested equal that may be 0,
// or indexed at. `v[n + 1]` past two pushes keeps its check too: two below
// the length is no relation the analysis keeps (ADR 1168).
use std::panic;

fn pushed(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v[n]
}

fn owned() -> u32 {
    let mut v = vec![1, 2];
    let n = v.len();
    v.push(3);
    v.push(4);
    v[n] * 10 + v[n + 1]
}

fn push(heap: &mut Vec<u32>, n: u32) {
    let index = heap.len();
    heap.push(n);
    sift_up(heap, index);
}

fn sift_up(heap: &mut [u32], i: usize) {
    let mut index = i;
    while index > 0 {
        let parent = (index - 1) >> 1;
        if heap[parent] > heap[index] {
            heap.swap(parent, index);
            index = parent;
        } else {
            return;
        }
    }
}

fn nested(properties: &mut Vec<u32>, more: bool) {
    if more {
        properties.push(1);
    }
}

fn diff(properties: &mut Vec<u32>, more: bool) -> Vec<u32> {
    properties.push(0);
    let prev_length = properties.len();
    nested(properties, more);
    if prev_length == properties.len() {
        properties[prev_length - 1] = 9;
    }
    properties.clone()
}

fn last_of_at_least_one(v: &[u32], n: usize) -> u32 {
    if n >= 1 && n == v.len() { v[n - 1] } else { 0 }
}

/// `n - 1` of `n = v.len()` before the push, 0 of an empty one: wraps.
fn one_before(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v[n - 1]
}

fn past_a_pop(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.pop();
    v[n - 1]
}

fn past_a_clear(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v.clear();
    v[n]
}

fn past_a_new_vector(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    *v = Vec::new();
    v[n]
}

fn another_pushed(a: &[u32], b: &mut Vec<u32>) -> u32 {
    let n = a.len();
    b.push(7);
    a[n]
}

fn wrapping_more(v: &mut Vec<u32>) -> u32 {
    let n = v.len().wrapping_add(1);
    v.push(7);
    v[n]
}

fn last_of_maybe_none(v: &[u32], n: usize) -> u32 {
    if n == v.len() { v[n - 1] } else { 0 }
}

/// What was at most the length, below it past a push.
fn at_most_pushed(v: &mut Vec<u32>, i: usize) -> u32 {
    if i <= v.len() {
        v.push(7);
        v[i]
    } else {
        0
    }
}

/// Equal to the length, which is no index.
fn equal_at(v: &[u32], n: usize) -> u32 {
    if n == v.len() { v[n] } else { 0 }
}

/// Below another slice's length, pushed onto this one.
fn below_another(a: &[u32], b: &mut Vec<u32>, i: usize) -> u32 {
    if i < a.len() {
        b.push(7);
        b[i]
    } else {
        0
    }
}

fn run(name: &str, row: impl FnOnce() -> u32 + panic::UnwindSafe) {
    match panic::catch_unwind(row) {
        Ok(value) => println!("{name}: {value}"),
        Err(_) => println!("{name}: panicked"),
    }
}

fn main() {
    run("pushed", || pushed(&mut vec![1, 2]));
    run("pushed of none", || pushed(&mut Vec::new()));
    run("owned", owned);
    let mut heap = Vec::new();
    for n in [5, 3, 8, 1, 4] {
        push(&mut heap, n);
    }
    println!("heap {heap:?}");
    println!("diff {:?}", diff(&mut vec![4], false));
    println!("diff with more {:?}", diff(&mut vec![4], true));
    run("last of at least one", || last_of_at_least_one(&[4, 5], 2));
    run("last of another length", || last_of_at_least_one(&[4, 5], 1));
    run("one before", || one_before(&mut vec![1, 2]));
    run("one before of none", || one_before(&mut Vec::new()));
    run("past a pop", || past_a_pop(&mut vec![1]));
    run("past a clear", || past_a_clear(&mut vec![1]));
    run("past a new vector", || past_a_new_vector(&mut vec![1]));
    run("another pushed", || another_pushed(&[1, 2], &mut vec![3]));
    run("wrapping more", || wrapping_more(&mut vec![1]));
    run("last of maybe none", || last_of_maybe_none(&[], 0));
    run("last of maybe none, one", || last_of_maybe_none(&[6], 1));
    run("at most pushed", || at_most_pushed(&mut vec![1, 2], 2));
    run("at most pushed, before", || at_most_pushed(&mut vec![1, 2], 1));
    run("equal at", || equal_at(&[1, 2], 2));
    run("below another", || below_another(&[1, 2, 3], &mut Vec::new(), 2));
}
