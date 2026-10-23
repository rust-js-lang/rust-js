// A typed payload, `resume_unwind(Box::new(v))` of a `v` that's not text,
// downcasts to its own type, as Rust's does, though it downcasts as the
// `&JsError` thrown too (ADR 0710): `is`, `downcast_ref`, `downcast_mut`
// and `Box::downcast` of it, resumed and caught again as it was, changed
// first; and a `Box<dyn Any + Send>` the program makes, never thrown.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

#[derive(Debug, PartialEq)]
struct Code {
    n: u32,
    why: &'static str,
}

fn thrown<T: Any + Send>(value: T) -> Box<dyn Any + Send> {
    panic::catch_unwind(AssertUnwindSafe(|| -> u32 { panic::resume_unwind(Box::new(value)) }))
        .expect_err("it unwound")
}

fn describe(payload: &(dyn Any + Send)) -> String {
    if let Some(n) = payload.downcast_ref::<i32>() {
        format!("an i32, {n}")
    } else if let Some(code) = payload.downcast_ref::<Code>() {
        format!("a Code, {code:?}")
    } else if payload.is::<f64>() {
        "an f64".to_string()
    } else {
        "something else".to_string()
    }
}

fn main() {
    println!("{}", describe(&*thrown(5i32)));
    println!("{}", describe(&*thrown(Code { n: 3, why: "lost" })));
    println!("{}", describe(&*thrown(1.5f64)));
    println!("{}", describe(&*thrown(true)));

    // Changed, then resumed: caught again as it is now.
    let mut payload = thrown(7i32);
    if let Some(n) = payload.downcast_mut::<i32>() {
        *n += 1;
    }
    let again = panic::catch_unwind(AssertUnwindSafe(|| -> u32 { panic::resume_unwind(payload) }));
    println!("{}", describe(&*again.expect_err("resumed")));

    // `Box::downcast` takes the value out, or gives the box back.
    match thrown(Code { n: 9, why: "taken" }).downcast::<Code>() {
        Ok(code) => println!("taken {}", code.why),
        Err(_) => println!("not a Code"),
    }
    match thrown(2u8).downcast::<i32>() {
        Ok(n) => println!("an i32 {n}"),
        Err(payload) => println!("a u8 kept: {}", payload.is::<u8>()),
    }

    // A box of the program's own, never thrown, is its value's.
    let own: Box<dyn Any + Send> = Box::new(Code { n: 1, why: "own" });
    println!("{} {}", describe(&*own), own.is::<i32>());
}
