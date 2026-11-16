// A binary heap's children, `left = (index + 1) * 2 - 1`, under `index <
// length >> 1`: below the heap's length, since twice what's below a half is
// below the whole, so `heap[left]` and `heap[index]` read with no check
// (ADR 1168, #1948). What only looks alike keeps its check and panics as
// Rust's does: a loop to `index <= half`, a right child `* 2 + 2` read
// under the left's bound, and an index past a wrapping add.
use std::panic;

fn sift_down(heap: &mut [u32], i: usize) {
    let mut index = i;
    let length = heap.len();
    let half_length = length >> 1;
    while index < half_length {
        let left_index = (index + 1) * 2 - 1;
        let right_index = left_index + 1;
        if heap[left_index] < heap[index] {
            if right_index < length && heap[right_index] < heap[left_index] {
                heap.swap(index, right_index);
                index = right_index;
            } else {
                heap.swap(index, left_index);
                index = left_index;
            }
        } else if right_index < length && heap[right_index] < heap[index] {
            heap.swap(index, right_index);
            index = right_index;
        } else {
            return;
        }
    }
}

fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    if heap.is_empty() {
        return None;
    }
    let first = heap.swap_remove(0);
    if !heap.is_empty() {
        sift_down(heap, 0);
    }
    Some(first)
}

/// A half by `/ 2`, and the left child `2 * index + 1`.
fn first_child_sum(heap: &[u32]) -> u32 {
    let half = heap.len() / 2;
    let mut sum = 0;
    let mut index = 0;
    while index < half {
        sum += heap[2 * index + 1];
        index += 1;
    }
    sum
}

/// Past the half: `index <= half` lets `left` reach the length.
fn past_the_half(heap: &[u32]) -> u32 {
    let half = heap.len() >> 1;
    let mut sum = 0;
    let mut index = 0;
    while index <= half {
        sum += heap[index * 2 + 1];
        index += 1;
    }
    sum
}

/// The right child, `* 2 + 2`, under the left's bound alone.
fn right_unguarded(heap: &[u32]) -> u32 {
    let half = heap.len() >> 1;
    let mut sum = 0;
    let mut index = 0;
    while index < half {
        sum += heap[index * 2 + 2];
        index += 1;
    }
    sum
}

/// Doubled past a wrapping add, which proves nothing.
fn wrapped(heap: &[u32], i: usize) -> u32 {
    let half = heap.len() >> 1;
    if i < half {
        let j = i.wrapping_add(usize::MAX);
        heap[j.wrapping_mul(2).wrapping_add(1)]
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
    let mut popped = Vec::new();
    let mut h = vec![1, 3, 2, 7, 4, 9, 5];
    while let Some(v) = pop(&mut h) {
        popped.push(v);
    }
    println!("{popped:?}");
    run("first children", || first_child_sum(&[1, 2, 3, 4, 5, 6, 7]));
    run("first children of six", || first_child_sum(&[1, 2, 3, 4, 5, 6]));
    run("past the half, odd", || past_the_half(&[1, 2, 3, 4, 5]));
    run("past the half, even", || past_the_half(&[1, 2, 3, 4]));
    run("right unguarded, odd", || right_unguarded(&[1, 2, 3, 4, 5]));
    run("right unguarded, even", || right_unguarded(&[1, 2, 3, 4]));
    run("wrapped", || wrapped(&[1, 2, 3, 4], 1));
    run("wrapped at zero", || wrapped(&[1, 2, 3, 4], 0));
}
