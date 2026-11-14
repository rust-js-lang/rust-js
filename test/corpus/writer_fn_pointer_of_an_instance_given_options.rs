// A generic writer function, given its `Formatter`, named as a `fn` pointer
// at one of its instances, in a crate whose writers take a `Formatter`'s
// options: the pointer gives them after its arguments, before its
// dictionaries, as a call through it does (#1758); one whose forms differ
// from its callers', a `&mut T`'s box, too.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn quoted<T: fmt::Display>(x: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "'{x}'")
}

fn bumped<T: fmt::Display>(n: &mut T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match f.alternate() {
        true => write!(f, "<<{n}>>"),
        false => write!(f, "<{n}>"),
    }
}

struct Point(i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

struct Bumped(i32);

impl fmt::Display for Bumped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bump: fn(&mut Point, &mut fmt::Formatter<'_>) -> fmt::Result = bumped;
        let mut p = Point(self.0);
        bump(&mut p, f)
    }
}

struct Joined<'a>(&'a [u32]);

impl Joined<'_> {
    fn format(&self, f: &mut fmt::Formatter<'_>, cb: fn(&u32, &mut fmt::Formatter<'_>) -> fmt::Result) -> fmt::Result {
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            cb(x, f)?;
        }
        Ok(())
    }
}

impl fmt::Display for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.format(f, fmt::Display::fmt)
        } else {
            self.format(f, quoted)
        }
    }
}

fn main() {
    let items = [1, 20];
    let mut log = Log(Vec::new());
    write!(log, "{}|{:#}", Joined(&items), Joined(&items)).unwrap();
    println!("{:?}", log.0);
    println!("{} {:>4}", Joined(&items), Joined(&[3]));
    write!(log, "{}", Bumped(6)).unwrap();
    println!("{} {:#} {:?}", Bumped(5), Bumped(7), log.0);
}
