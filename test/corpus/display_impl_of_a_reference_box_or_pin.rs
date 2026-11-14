//@ library-differs
// Formatting impls of a reference to, a `Box` of and a `Pin<Box<..>>` of
// the crate's own types, which the orphan rules allow as `&`, `Box` and
// `Pin` are fundamental: each formats by its own impl, plainly, with
// options, by `to_string()`, written into a writer of the crate's own a
// piece at a time, directly, through generic code and through a `dyn`, and
// shown by another writer (#1798).
use std::fmt::{self, Write};
use std::pin::Pin;

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct ByRef(u8);

impl fmt::Display for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

impl fmt::Debug for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ByRef").field(&self.0).finish()
    }
}

struct Boxed(u8);

impl fmt::Display for Box<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}", self.0)
    }
}

struct Pinned(u8);

impl fmt::Display for Pin<Box<Pinned>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

struct Outer<'a>(&'a ByRef, Box<Boxed>);

impl fmt::Display for Outer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}|{}>", self.0, self.1)
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

fn shown<T: fmt::Display>(t: T) -> String {
    format!("{t:>6}")
}

fn main() {
    let r = ByRef(1);
    let b = Box::new(Boxed(2));
    let p = Box::pin(Pinned(3));
    println!("{} {} {}", &r, b, p);
    println!("{:?} {:#?}", &r, &r);
    println!("[{:>5}] [{:<5}] [{:^7}]", &r, b, p);
    println!("{} {} {}", (&r).to_string(), b.to_string(), p.to_string());
    println!("{} {}", shown(&r), shown(&b));

    let mut log = Log(Vec::new());
    write!(log, "{}", &r).unwrap();
    write!(log, "{}", b).unwrap();
    write!(log, "{}", p).unwrap();
    own(&mut log, &&r).unwrap();
    own(&mut log, &b).unwrap();
    own(&mut log, &p).unwrap();
    let dyns: [&dyn fmt::Display; 3] = [&&r, &b, &p];
    for d in dyns {
        write!(log, "{d}").unwrap();
    }
    write!(log, "{}", Outer(&r, Box::new(Boxed(4)))).unwrap();
    println!("{:?}", log.0);
}
