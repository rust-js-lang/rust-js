//@ run-fail: checked: Custom { kind: InvalidInput, error: "too big" }
// `expect` of an `io::Result<()>` that's an `Err`: std's panic, of the
// error's `{:?}` (ADR 0462).
use std::io;

fn check(n: u32) -> io::Result<()> {
    if n > 3 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "too big"));
    }
    Ok(())
}

fn main() {
    check(1).expect("checked");
    println!("one was fine");
    check(9).expect("checked");
}
