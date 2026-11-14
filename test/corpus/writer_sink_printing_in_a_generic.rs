//@ library-differs
// A type whose `Debug` prints as it formats, shown by a generic type of
// the crate's own whose writer writes a piece at a time, as it's written
// into a writer elsewhere (#1712): its dictionary carries `fmtInto`, so
// each print lands where Rust's stdout's line has it, between the pieces
// `{:#?}`'s `PadAdapter` writes, not inside its own text (#1608); and a
// `Vec` of it, whose dictionary carries `fmtInto` so.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
struct Wrap<T> {
    inner: T,
}

struct Stepped(Vec<u32>);

impl fmt::Debug for Stepped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|n| {
                print!("<step {n}>");
                n
            }))
            .finish()
    }
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{:?}", Wrap { inner: 1 }).unwrap();
    println!("{:?}", log.0);
    println!("x {:#?} y", Wrap { inner: Stepped(vec![1, 2]) });
    println!("x {:?} y", Wrap { inner: Stepped(vec![3]) });
    println!("x {:#?} y", Wrap { inner: vec![Stepped(vec![4]), Stepped(vec![5, 6])] });
}
