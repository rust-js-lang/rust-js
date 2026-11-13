// A function that gives back a write's result, `fmt::Result` or
// `io::Result<()>`, whose `Err` comes before a drop that may panic, one that
// formats: the drops run, then the `Err` is given back, as Rust's return
// runs them (#1636). Where no drop may panic, a `finally` runs them.
use std::fmt::{self, Write};
use std::io;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        said(format_args!("drop {}", self.0));
    }
}

fn said(what: fmt::Arguments) {
    let text = what.to_string();
    println!("{text}");
}

struct Failing;

impl fmt::Debug for Failing {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

impl Drop for Failing {
    fn drop(&mut self) {
        said(format_args!("drop Failing"));
    }
}

fn made(n: u8) -> fmt::Result {
    let _loud = Loud("made");
    if n > 1 {
        return Err(fmt::Error);
    }
    Ok(())
}

fn written(fails: bool) -> fmt::Result {
    let _loud = Loud("written");
    let mut text = String::new();
    match fails {
        true => write!(text, "{:?}", Failing),
        false => write!(text, "fine"),
    }
}

fn passed(n: u8) -> fmt::Result {
    let _loud = Loud("passed");
    made(n)
}

fn tried(n: u8) -> fmt::Result {
    let _loud = Loud("tried");
    made(n)?;
    println!("tried on");
    Ok(())
}

fn io_made(fails: bool) -> io::Result<()> {
    let _loud = Loud("io");
    match fails {
        true => Err(io::Error::other("io failed")),
        false => Ok(()),
    }
}

fn main() {
    println!("{:?} {:?}", made(1), made(2));
    println!("{:?} {:?}", written(false), written(true));
    println!("{:?} {:?}", passed(1), passed(2));
    println!("{:?} {:?}", tried(1), tried(2));
    println!("{:?}", io_made(false));
    println!("{:?}", io_made(true).map_err(|e| e.to_string()));
}
