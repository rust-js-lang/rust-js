//@ library-differs
// `Debug` impls, and a `Display` of a `&mut`, of a reference to, a `Box`
// of and a `Pin<Box<..>>` of the crate's own types: each formats by its own
// impl, never through what it holds as std's would, in a derived `Debug`'s
// field, a `Vec`'s and an `Option`'s items, with options, through generic
// code and a `dyn Debug`, and written into a writer of the crate's own a
// piece at a time (#1798).
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

impl fmt::Debug for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0)
    }
}

impl fmt::Display for &mut ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m{}", self.0)
    }
}

#[derive(Debug)]
struct Holder<'a> {
    by: &'a ByRef,
    n: i32,
}

struct Boxed(u8);

impl fmt::Debug for Box<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Boxed").field("v", &self.0).finish()
    }
}

impl fmt::Display for Box<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("boxed")
    }
}

struct Pinned(u8);

impl fmt::Debug for Pin<Box<Pinned>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

fn debug_of<T: fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn main() {
    let r = ByRef(1);
    let h = Holder { by: &r, n: 2 };
    println!("{:?}", h);
    println!("{:#?}", h);
    let v = vec![&r, &r];
    println!("{:?} {}", v, debug_of(&v));
    let boxes = vec![Box::new(Boxed(3)), Box::new(Boxed(4))];
    println!("{:?}", boxes);
    println!("[{:>8}] [{:<7}] {}", boxes[0], boxes[1], boxes[0].to_string());
    let o = Some(Box::new(Boxed(5)));
    println!("{:?}", o);
    let p = Box::pin(Pinned(6));
    println!("{:?} {}", p, debug_of(&p));
    let mut m = ByRef(7);
    let mr = &mut m;
    println!("{}", mr);
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:?}|{:?}|{}", h, boxes, p, mr).unwrap();
    let d: &dyn fmt::Debug = &boxes[1];
    write!(log, "{d:?}").unwrap();
    println!("{:?}", log.0);
    println!("{}", m.0);
}
