//@ run-fail: boom at 2
// A function that gives a value's `Result` and `?`s an `io::Result<()>`
// catches the `io::Error` it throws, its `Err`, and nothing else: a panic
// in it is a panic (ADR 0462).
use std::io;

fn check(n: u32) -> io::Result<()> {
    if n > 3 {
        return Err(io::Error::other("too big"));
    }
    Ok(())
}

fn risky(n: u32) -> io::Result<u32> {
    check(n)?;
    if n == 2 {
        panic!("boom at {n}");
    }
    Ok(n)
}

fn main() {
    println!("{:?}", risky(9).map_err(|e| e.to_string()));
    println!("{:?}", risky(1).is_ok());
    let _ = risky(2);
}
