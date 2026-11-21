// An `io::Error` and its `ErrorKind` written into a writer of the crate's
// own a piece at a time, as std's `fmt` writes them (#1420): by `{}`,
// padded or not, by `{:?}` and `{:#?}` as a derived `Debug`'s fields,
// and by a `Display`'s `write!`, rand 0.7.3's `ReadError`, each write a
// call of the writer's, so one that fails stops the rest where Rust's does.
use std::fmt::{self, Write};
use std::io;

struct Counted {
    text: String,
    calls: u32,
    fail_on: Option<&'static str>,
}

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.calls += 1;
        if self.fail_on.is_some_and(|bad| s.contains(bad)) {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

#[derive(Debug)]
struct ReadError(io::Error);

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ReadError: {}", self.0)
    }
}

impl std::error::Error for ReadError {}

#[derive(Debug)]
struct Holder {
    kind: io::ErrorKind,
    error: io::Error,
}

fn counted(fail_on: Option<&'static str>) -> Counted {
    Counted { text: String::new(), calls: 0, fail_on }
}

fn main() {
    let errors = [
        io::Error::new(io::ErrorKind::Other, "boom"),
        io::Error::from(io::ErrorKind::NotFound),
        io::Error::new(io::ErrorKind::InvalidData, ReadError(io::Error::new(io::ErrorKind::Other, "inner"))),
    ];
    for error in errors {
        let mut c = counted(None);
        let r = write!(c, "[{}|{:>12}|{:?}]", error, error, error);
        println!("{:?} {} {:?}", c.text, c.calls, r);
        let holder = Holder { kind: error.kind(), error };
        let mut c = counted(None);
        let r = write!(c, "{:?} {:#?} {} {:?}", holder, holder, holder.kind, holder.kind);
        println!("{:?} {} {:?}", c.text, c.calls, r);
        let mut c = counted(Some("kind"));
        let r = write!(c, "{:?}", holder);
        println!("{:?} {} {:?}", c.text, c.calls, r);
    }
    let read = ReadError(io::Error::new(io::ErrorKind::Other, "x"));
    let mut c = counted(Some("x"));
    let r = write!(c, "{} / {:?}", read, read);
    println!("{:?} {} {:?}", c.text, c.calls, r);
    let boxed: Box<dyn std::error::Error> = Box::new(ReadError(io::Error::from(io::ErrorKind::TimedOut)));
    let mut c = counted(None);
    let r = write!(c, "{} {:?}", boxed, boxed);
    println!("{:?} {} {:?}", c.text, c.calls, r);
}
