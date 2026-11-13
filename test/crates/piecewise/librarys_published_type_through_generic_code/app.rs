//! Writes the library's types into a writer through generic code, and
//! directly into a writer of its own, a piece at a time by the dictionary
//! its library publishes.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "<{}>", t)
}

// Writes the library's list by its writer function, given its `Formatter`.
struct Mine(dep::Listed);

impl fmt::Display for Mine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        self.0.joined(f, "|")?;
        f.write_str(">")
    }
}

pub fn main() {
    let mut text = String::new();
    own(&mut text, &dep::Meters(6)).unwrap();
    let mut log = Log(Vec::new());
    own(&mut log, &dep::Meters(7)).unwrap();
    write!(log, "{}|", dep::Meters(8)).unwrap();
    println!("{} {:?}", text, log.0);
    let name = String::from("ab");
    let mut log = Log(Vec::new());
    own(&mut log, &dep::Prefixed(&name, -4)).unwrap();
    write!(log, "{}|", dep::Prefixed("c", 5)).unwrap();
    println!("{:?} {}", log.0, dep::Prefixed("d", 6));
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:#?}", dep::Pair(1, "a\""), dep::Pair(-2, "b")).unwrap();
    own(&mut log, &format!("{:?}", dep::Pair(3, "c"))).unwrap();
    println!("{:?}", log.0);
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:?}", dep::der(true), dep::der(false)).unwrap();
    let anchor = dep::Anchor {
        subject: dep::der(true),
        constraints: Some(dep::der(false)),
    };
    write!(log, "|{:?}|{:#?}", anchor, anchor.clone()).unwrap();
    println!("{:?}", log.0);
    let mut log = Log(Vec::new());
    write!(log, "{}", Mine(dep::Listed(vec![1, 20, 3]))).unwrap();
    println!("{:?}", log.0);
}
