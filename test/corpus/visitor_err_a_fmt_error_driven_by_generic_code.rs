//@ library-differs
// #2057 (ADR 0461, amended): a trait whose methods give `Result<_,
// Self::Err>`, generic code's `{ TAG }`, implemented by a printer whose
// `Err` is `fmt::Error`, as regex-syntax's `impl Visitor for Writer<W>`:
// its dictionary gives each `fmt::Result` tagged, a written method's, one
// whose `Ok` is `Self::Output`, `finish`'s, and a default it doesn't
// write, `visit_between`'s `Ok(())`, each; native and JS print the same.
use std::fmt::{self, Write};

trait Visitor {
    type Output;
    type Err;

    fn start(&mut self) {}

    fn visit(&mut self, item: u32) -> Result<(), Self::Err>;

    fn visit_between(&mut self) -> Result<(), Self::Err> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Output, Self::Err>;
}

fn walk<V: Visitor>(items: &[u32], mut visitor: V) -> Result<V::Output, V::Err> {
    visitor.start();
    for (i, &item) in items.iter().enumerate() {
        if i > 0 {
            visitor.visit_between()?;
        }
        visitor.visit(item)?;
    }
    visitor.finish()
}

struct Printer<W> {
    out: W,
}

impl<W: Write> Visitor for Printer<W> {
    type Output = ();
    type Err = fmt::Error;

    fn visit(&mut self, item: u32) -> fmt::Result {
        write!(self.out, "<{item}>")
    }

    fn finish(self) -> fmt::Result {
        Ok(())
    }
}

struct Limited {
    text: String,
    room: usize,
}

impl Write for Limited {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.text.len() + s.len() > self.room {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

fn main() {
    let mut text = String::new();
    let printed = walk(&[1, 22, 333], Printer { out: &mut text });
    println!("{printed:?} {text}");
    let mut limited = Limited {
        text: String::new(),
        room: 8,
    };
    let printed = walk(&[1, 22, 333], Printer { out: &mut limited });
    println!("{printed:?} {}", limited.text);
}
