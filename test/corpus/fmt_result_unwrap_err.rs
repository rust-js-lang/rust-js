// `unwrap_err()` and `expect_err(..)` of a write's `fmt::Result`: the
// `fmt::Error` where the write failed, std's panic where it didn't (#1160),
// chrono's `write!(buf, "{}", dt.format("%#z")).expect_err(..)`.
use std::any::Any;
use std::fmt::{self, Write};
use std::panic::{self, AssertUnwindSafe};

struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("half")?;
        Err(fmt::Error)
    }
}

struct Fine;

impl fmt::Display for Fine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("whole")
    }
}

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
    let error = write!(buf, "{}", Refused).expect_err("a refused write fails");
    println!("{error:?} {error} [{buf}]");
    let _ = write!(buf, "{}", Refused).unwrap_err();
    println!("[{buf}]");
    let fine = panic::catch_unwind(AssertUnwindSafe(|| {
        write!(buf, "{}", Fine).expect_err("a fine write fails")
    }));
    println!("{} [{buf}]", fine.map(|_| ()).map_err(message).unwrap_err());
    let fine = panic::catch_unwind(AssertUnwindSafe(|| write!(buf, "{}", Fine).unwrap_err()));
    println!("{} [{buf}]", fine.map(|_| ()).map_err(message).unwrap_err());
}
