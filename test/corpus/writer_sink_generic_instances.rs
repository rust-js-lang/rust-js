//@ library-differs
// A generic type of the crate's own written into a writer, so its writer
// writes a piece at a time and writes its type parameters by their
// dictionaries' `fmtInto` (ADR 1135), and formatted elsewhere with other
// arguments: each instance the crate asks for gives its parameters
// dictionaries that carry it (#1738). A `{}` of an instance, one a generic
// caller makes, a `Box<dyn Display>` and a `&dyn Display` of one, its
// `to_string()`, one holding a type of the crate's whose writer is then
// framed too, and one nested in itself, written and printed (#1739); and a
// derived `Debug`'s.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Wrap<T>(T);

impl<T: fmt::Display> fmt::Display for Wrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

struct Point(i32, i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

#[derive(Debug)]
struct Pair<T> {
    left: T,
    right: T,
}

fn wrapped<T: fmt::Display>(t: T) -> String {
    format!("{}", Wrap(t))
}

fn boxed<T: fmt::Display + 'static>(t: T) -> Box<dyn fmt::Display> {
    Box::new(Wrap(t))
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "[{}]", Wrap(3)).unwrap();
    write!(log, "[{}]", Wrap(Wrap(1.5))).unwrap();
    write!(log, "{:?}", Pair { left: 1u8, right: 2 }).unwrap();
    println!("{:?}", log.0);
    println!("{}", Wrap(String::from("s")));
    println!("{}", Wrap(Point(1, 2)));
    println!("{}", wrapped('c'));
    println!("{}", wrapped(Point(3, 4)));
    let shown: Box<dyn fmt::Display> = Box::new(Wrap(true));
    println!("{shown}");
    let lent: &dyn fmt::Display = &Wrap(0.25f32);
    println!("{lent}");
    let made = boxed(-7i64);
    println!("{made}");
    println!("{}", Wrap(5u8).to_string());
    println!("{}", Wrap(Wrap("in")));
    println!("{:?}", Pair { left: "a", right: "b" });
}
