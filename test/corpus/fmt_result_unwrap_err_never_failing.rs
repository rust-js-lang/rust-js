// `unwrap_err()` and `expect_err(..)` of a write's `fmt::Result` in a crate
// whose writes never fail: std's panic, after the write (#1160).
use std::any::Any;
use std::fmt::Write;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn main() {
    let mut buf = String::new();
    let label = String::from("a write fails");
    let fine = panic::catch_unwind(AssertUnwindSafe(|| write!(buf, "{}", 1).expect_err(&label)));
    println!("{} [{buf}]", fine.map(|_| ()).map_err(message).unwrap_err());
    let fine = panic::catch_unwind(AssertUnwindSafe(|| write!(buf, "{}", 2).unwrap_err()));
    println!("{} [{buf}]", fine.map(|_| ()).map_err(message).unwrap_err());
}
