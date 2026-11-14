// Two `resume_unwind`s a macro's repetition writes, of one span: each a
// payload caught and resumed, thrown again as Rust resumes it (rd550,
// #1671 change 3 M5): a fact of one of them is asked of the call itself,
// never of a span another shares.
use std::panic::{catch_unwind, resume_unwind};

fn inner(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    if n == 0 {
        resume_unwind(Box::new(7i32));
    }
    n
}

macro_rules! each_again {
    ($($n:expr),*) => {{
        let mut total = 0;
        $(
            match catch_unwind(|| inner($n)) {
                Ok(v) => total += v,
                Err(payload) => resume_unwind(payload),
            }
        )*
        total
    }};
}

fn again(n: u32) -> u32 {
    each_again!(n, n + 1)
}

fn main() {
    for n in [1, 3, 0, 2] {
        match catch_unwind(|| again(n)) {
            Ok(v) => println!("ok {v}"),
            Err(payload) => {
                if let Some(message) = payload.downcast_ref::<String>() {
                    println!("panic {message}");
                } else if let Some(n) = payload.downcast_ref::<i32>() {
                    println!("payload {n}");
                } else {
                    println!("other");
                }
            }
        }
    }
}
