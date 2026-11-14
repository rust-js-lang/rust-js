//! Writes types that hold the library's `Token`, which nothing formats,
//! into a writer through generic code, a piece at a time: the consumer's
//! own, whose `Display` never shows it, and the library's `Tag`.
use std::fmt::{self, Write};
use std::marker::PhantomData;

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Labeled<T>(T, &'static str);

impl<T> fmt::Display for Labeled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
    }
}

struct Marked<T>(PhantomData<T>, i32);

impl<T> fmt::Display for Marked<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m{}", self.1)
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

pub fn main() {
    let mut log = Log(Vec::new());
    let labeled = Labeled(dep::Token(1), "tok");
    own(&mut log, &labeled).unwrap();
    own(&mut log, &Marked::<dep::Token>(PhantomData, 2)).unwrap();
    own(&mut log, &dep::Tag(dep::Token(3))).unwrap();
    own(&mut log, &dep::Tag(dep::Shown(4))).unwrap();
    own(&mut log, &dep::Shown(5)).unwrap();
    println!("{:?} {} {}", log.0, labeled.0.0, dep::Token(6).0);
}
