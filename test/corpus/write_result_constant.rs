// A write's result rustc computed, a `const` or a promoted `&Ok(())`, is
// the write's result it is (#1411): its `Ok` nothing and its `Err` a write's
// error, read as one, given back by a function, by a `Display`, and by one
// that gives back what it holds.
use std::fmt::{self, Write as _};
use std::io;

const DONE: fmt::Result = Ok(());
const FAILED: fmt::Result = Err(fmt::Error);

fn ok(r: &fmt::Result) -> bool {
    r.is_ok()
}

fn io_ok(r: &io::Result<()>) -> bool {
    r.is_ok()
}

fn given(fail: bool) -> fmt::Result {
    if fail { FAILED } else { DONE }
}

struct Partly(&'static str);

impl fmt::Display for Partly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)?;
        FAILED
    }
}

struct Held(bool);

impl fmt::Display for Held {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("held")?;
        let r = if self.0 { Err(fmt::Error) } else { Ok(()) };
        r
    }
}

fn main() {
    println!("{} {}", ok(&Ok(())), ok(&Err(fmt::Error)));
    println!("{} {} {:?} {:?}", ok(&DONE), ok(&FAILED), DONE, FAILED);
    let r: &fmt::Result = &Ok(());
    println!("{} {r:?} {}", r.is_ok(), io_ok(&Ok(())));
    println!("{:?} {:?}", given(true), given(false));

    // A `Display` that gives back what it was given, or what it holds.
    for text in ["abc", "de"] {
        let mut s = String::new();
        println!("{:?} {s:?}", write!(s, "<{}>", Partly(text)));
    }
    for fail in [false, true] {
        let mut s = String::new();
        println!("{:?} {s:?}", write!(s, "{}", Held(fail)));
    }
}
