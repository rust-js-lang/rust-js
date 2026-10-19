// A `Formatter` kept in a struct that generic code writes to, as base64's
// `Display` gives its encoder a `FormatterSink`: a handle on the writer's
// text, written as a `String`'s is. A `fmt::Result` given where generic
// code has a `Result<(), S::Error>`, and back, its `Err` the same; and a
// writer's last call that fails, failing it (ADR 0461).
use std::fmt::{self, Write};

trait Sink {
    type Error;
    fn write_encoded(&mut self, encoded: &str) -> Result<(), Self::Error>;
}

fn encode<S: Sink>(text: &str, sink: &mut S) -> Result<(), S::Error> {
    for word in text.split(' ') {
        sink.write_encoded(word)?;
    }
    Ok(())
}

struct FormatterSink<'a, 'b: 'a> {
    f: &'a mut fmt::Formatter<'b>,
}

impl Sink for FormatterSink<'_, '_> {
    type Error = fmt::Error;
    fn write_encoded(&mut self, encoded: &str) -> fmt::Result {
        if encoded == "bad" {
            return Err(fmt::Error);
        }
        self.f.write_str(encoded)?;
        self.f.write_char('.')
    }
}

struct Encoded<'a>(&'a str);

impl fmt::Display for Encoded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut sink = FormatterSink { f };
        encode(self.0, &mut sink)
    }
}

struct StringSink<'a> {
    s: &'a mut String,
}

impl Sink for StringSink<'_> {
    type Error = fmt::Error;
    fn write_encoded(&mut self, encoded: &str) -> fmt::Result {
        write!(self.s, "<{encoded}>")
    }
}

fn check(n: u32) -> fmt::Result {
    if n > 3 {
        return Err(fmt::Error);
    }
    Ok(())
}

struct Checked(u32);

impl fmt::Display for Checked {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)?;
        check(self.0)
    }
}

fn main() {
    println!("{}", Encoded("a b c"));
    let mut out = String::new();
    let failed = write!(out, "[{}]", Encoded("a bad c"));
    println!("{failed:?} {out:?}");

    let mut s = String::new();
    let written = encode("x y", &mut StringSink { s: &mut s });
    println!("{written:?} {s}");

    let mut checked = String::new();
    let fine = write!(checked, "{}", Checked(2));
    let not = write!(checked, "{}", Checked(5));
    println!("{fine:?} {not:?} {checked:?}");
}
