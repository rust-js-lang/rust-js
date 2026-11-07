// A call that never returns, of type `!`, ends the path as a `return`
// does (#867): past `if n > 100 { panic!(..) }`, `n` is at most 100, so
// `n + 1` can't wrap and is written without it. Each guard is reached
// with 100, 101 and `u32::MAX`, items of an array the ranges know nothing
// of, and its panic caught: only the path the guard narrows goes on, and
// past the `catch_unwind` around it, `n + 1` of `u32::MAX` wraps to 0.
use std::panic;

fn guarded(n: u32) -> u32 {
    if n > 100 {
        panic!("too big: {n}");
    }
    n + 1
}

fn too_big(n: u32) -> ! {
    panic!("too big, through a call: {n}")
}

fn through_a_call(n: u32) -> u32 {
    if n > 100 {
        too_big(n);
    }
    n + 1
}

fn past_unreachable(n: u32) -> u32 {
    let small = if n <= 100 {
        n
    } else {
        unreachable!("past 100: {n}")
    };
    small + 1
}

// The guard's panic is the loop's only way out but `stop`: `i += 1`
// can't wrap, and `u32::MAX` panics where a wrap would have reached
// `stop`, 0.
fn count_to(start: u32, stop: u32) -> u32 {
    let mut i = start;
    loop {
        if i == stop {
            return i;
        }
        if i > 100 {
            panic!("ran past 100: {i}");
        }
        i += 1;
    }
}

// A loop no `break` leaves, whose only way out is its panic.
fn never_ends(start: u32) -> u32 {
    let mut i = start;
    loop {
        if i > 100 {
            panic!("never ends: {i}");
        }
        i += 1;
    }
}

fn main() {
    for n in [100, 101, u32::MAX] {
        println!("guarded {n} {:?}", panic::catch_unwind(|| guarded(n)).ok());
        println!("through a call {n} {:?}", panic::catch_unwind(|| through_a_call(n)).ok());
        println!("past unreachable {n} {:?}", panic::catch_unwind(|| past_unreachable(n)).ok());
        println!("count to {n} {:?}", panic::catch_unwind(|| count_to(n, 0)).ok());
        println!("count to {n} {:?}", panic::catch_unwind(|| count_to(n, 101)).ok());
        println!("never ends {n} {:?}", panic::catch_unwind(|| never_ends(n)).ok());
        let caught = panic::catch_unwind(|| {
            if n > 100 {
                panic!("too big in a closure: {n}");
            }
            n + 1
        });
        let past = n + 1;
        println!("closure {n} {:?}, past it {past}", caught.ok());
    }
}
