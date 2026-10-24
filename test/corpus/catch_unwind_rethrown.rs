// A payload caught and resumed is thrown as it was caught, the same
// object (ADR 0706): a panic's, caught again its message, and one of the
// program's own, caught again as it was; and a panic of no message,
// `panic!("")`, caught, its message empty.
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

fn again(n: u32) -> u32 {
    match catch_unwind(|| inner(n)) {
        Ok(v) => v,
        Err(payload) => resume_unwind(payload),
    }
}

fn main() {
    for n in [1, 3, 0] {
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
    let Err(empty) = catch_unwind(|| panic!(""));
    println!("empty {:?}", empty.downcast_ref::<&str>());
}
