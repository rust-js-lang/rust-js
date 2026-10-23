// `catch_unwind` catches a panic, its payload its message (ADR 0660): a
// literal one's, a formatted one's, std's own of an index, an `unwrap()`,
// a division; a payload resumed, of text or of a number, caught again; a
// `&mut` it's given through `AssertUnwindSafe`, changed before the panic.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

fn text(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(n) = payload.downcast_ref::<i32>() {
        format!("an i32, {n}")
    } else {
        "Box<dyn Any>".to_string()
    }
}

fn caught<T: std::fmt::Debug>(name: &str, result: std::thread::Result<T>) {
    match result {
        Ok(value) => println!("{name}: Ok({value:?})"),
        Err(payload) => println!("{name}: Err({})", text(&*payload)),
    }
}

fn pick(items: &[u32], at: usize) -> u32 {
    items[at]
}

fn halve(n: i32, by: i32) -> i32 {
    n / by
}

fn main() {
    caught("returns", panic::catch_unwind(|| 5));
    caught("literal", panic::catch_unwind(|| -> u32 { panic!("boom") }));
    let n = 7;
    caught("formatted", panic::catch_unwind(|| -> u32 { panic!("too big: {n}") }));
    caught("index", panic::catch_unwind(|| pick(&[1, 2], 5)));
    caught("unwrap", panic::catch_unwind(|| None::<u32>.unwrap()));
    caught("parse", panic::catch_unwind(|| "x".parse::<i32>().unwrap()));
    caught("divide", panic::catch_unwind(|| halve(n, n - 7)));

    // What's resumed is caught again, as itself.
    let inner = panic::catch_unwind(|| -> u32 { panic!("inner {}", 1) });
    let payload = inner.expect_err("it panicked");
    caught("resumed", panic::catch_unwind(AssertUnwindSafe(|| -> u32 { panic::resume_unwind(payload) })));
    caught("a number", panic::catch_unwind(|| -> u32 { panic::resume_unwind(Box::new(5i32)) }));

    // Caught inside, it's the outer one's value.
    let nested = panic::catch_unwind(|| match panic::catch_unwind(|| -> u32 { panic!("deep") }) {
        Ok(v) => v,
        Err(_) => 9,
    });
    caught("nested", nested);

    // The closure runs once, and what it changed stays changed.
    let mut count = 0;
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        count += 1;
        if count > 0 {
            panic!("counted");
        }
        count += 100;
    }));
    println!("count {count}, {}", result.is_err());

    // `Box::downcast` takes the message out.
    let owned = match panic::catch_unwind(|| -> u32 { panic!("owned {}", 2) }) {
        Ok(_) => String::new(),
        Err(payload) => match payload.downcast::<String>() {
            Ok(message) => *message,
            Err(payload) => text(&*payload),
        },
    };
    println!("{owned}");
}
