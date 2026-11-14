// `resume_unwind`s a macro's repetition writes, of one span: each a payload
// caught and resumed, thrown again as Rust resumes it (rd550, #1671 change
// 3 M5). Two alike are each of a payload as it was given; of two apart,
// one of a payload made there, neither's fact is taken of the other's.
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

macro_rules! resume_where {
    ($n:expr; $($when:literal => $payload:expr),*) => {
        $(
            if $n == $when {
                resume_unwind($payload);
            }
        )*
    };
}

fn again_or(n: u32) -> u32 {
    match catch_unwind(|| inner(n)) {
        Ok(v) => v,
        Err(payload) => {
            resume_where!(n; 3 => Box::new(9i32), 0 => payload);
            unreachable!()
        }
    }
}

fn report(caught: std::thread::Result<u32>) {
    match caught {
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

fn main() {
    for n in [1, 3, 0, 2] {
        report(catch_unwind(|| again(n)));
    }
    for n in [1, 3, 0] {
        report(catch_unwind(|| again_or(n)));
    }
}
