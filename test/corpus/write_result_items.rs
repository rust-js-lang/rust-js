// A write's result as an iterator's item (#1410): a callback that gives one
// gives it as a value, an item, not its error thrown past what steps it;
// std's code that reads each as a `Result`, a collect into one, is given it
// tagged, `{ TAG }`, and a write's result it collects into is given back as
// one; `flatten()` takes each `Ok`; `try_for_each` into an `io::Result<()>`
// is each item's call, its error the function's.
use std::fmt::{self, Write as _};
use std::io::{self, Write};

struct Fails;

impl io::Write for Fails {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("refused"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn lines(out: &mut impl Write, words: &[&str]) -> io::Result<()> {
    words.iter().map(|w| writeln!(out, "{w}")).collect()
}

fn each(out: &mut impl Write, words: &[&str]) -> io::Result<()> {
    words.iter().try_for_each(|w| writeln!(out, "{w}"))
}

fn main() {
    // Collected into one write's result, or each written in turn.
    let mut v = Vec::new();
    println!("{:?}", lines(&mut v, &["a", "b"]));
    println!("{:?}", lines(&mut Fails, &["a", "b"]));
    println!("{:?} {:?}", each(&mut v, &["c"]), each(&mut Fails, &["c"]));
    let all: io::Result<Vec<()>> = vec![writeln!(v, "e"), writeln!(Fails, "f")].into_iter().collect();
    println!("{}", all.is_err());
    let mut s = String::new();
    let r: fmt::Result = ["x", "y"].iter().map(|w| write!(s, "{w}")).collect();
    println!("{r:?} {s}");

    // Held, filtered, flattened and stepped, each item a value.
    let oks = vec![writeln!(v, "g"), writeln!(Fails, "h")];
    println!("{}", oks.iter().filter(|r| r.is_ok()).count());
    let firsts: Vec<()> = oks.into_iter().flatten().collect();
    println!("{}", firsts.len());
    let made: Vec<io::Result<()>> = ["i", "j"].iter().map(|w| writeln!(Fails, "{w}")).collect();
    println!("{} {}", made.len(), made.iter().all(|r| r.is_err()));
    let mut fails = 0;
    for r in ["k", "l"].iter().map(|w| writeln!(Fails, "{w}")) {
        if r.is_err() {
            fails += 1;
        }
    }
    println!("{fails}");
    let oks = ["m", "n"].iter().map(|w| writeln!(v, "{w}")).filter(Result::is_ok).count();
    println!("{oks}");
    let boxed: Box<dyn FnMut(&&str) -> io::Result<()>> = Box::new(|w| writeln!(Fails, "{w}"));
    println!("{}", ["q", "r"].iter().map(boxed).filter(|r| r.is_err()).count());
}
