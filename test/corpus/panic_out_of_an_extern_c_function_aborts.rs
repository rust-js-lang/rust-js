//@ run-abort
// A panic out of a function whose ABI can't unwind, `extern "C"`'s, aborts
// the process, as Rust's does, `catch_unwind` around it or not: std's
// `thread caused non-unwinding panic. aborting.`, exit 134 (#1582). One of
// `extern "C-unwind"` unwinds as any panic does, and an `extern "C"` one
// that can't panic is the function it is.
use std::panic::catch_unwind;

extern "C" fn doubled(n: i64) -> i64 {
    n * 2
}

extern "C-unwind" fn unwinding(n: i64) -> i64 {
    if n > 1 {
        panic!("unwinds at {n}");
    }
    n
}

extern "C" fn sum(count: i32) -> i64 {
    let mut total = 0i64;
    for i in 0..count {
        if i == 2 {
            panic!("at {i}");
        }
        total += i64::from(i);
    }
    total
}

fn main() {
    println!("doubled {}", doubled(21));
    let unwound = catch_unwind(|| unwinding(2));
    println!("unwound {}", unwound.is_err());
    let caught = catch_unwind(|| sum(3));
    println!("never here {}", caught.is_err());
}
