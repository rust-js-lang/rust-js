// A `Formatter` held where it isn't its writer's, of a writer that may
// print as it formats (#1637): the writer writes through a sink, and the
// handle a struct holds, or generic `fmt::Write` code is given, is that
// sink, so what's written through it reaches the line in its place, before
// what's printed after it, as Rust's stdout has it.
use std::fmt::{self, Debug, Display, Write};

// anyhow's `Quoted`, whose drop prints.
struct Quoted<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl Write for Quoted<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Display::fmt(&s.escape_debug(), self.0)
    }
}

impl Drop for Quoted<'_, '_> {
    fn drop(&mut self) {
        println!("drop Quoted");
    }
}

struct Message(String);

impl Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        Quoted(f).write_fmt(format_args!("{}", self.0))?;
        f.write_char('"')
    }
}

// base64's `FormatterSink`, written by generic code, whose drop prints.
trait Sink {
    fn write_encoded(&mut self, encoded: &str) -> fmt::Result;
}

fn encode<S: Sink>(text: &str, sink: &mut S) -> fmt::Result {
    for word in text.split(' ') {
        sink.write_encoded(word)?;
    }
    Ok(())
}

struct FormatterSink<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
}

impl Sink for FormatterSink<'_, '_> {
    fn write_encoded(&mut self, encoded: &str) -> fmt::Result {
        self.f.write_str(encoded)?;
        self.f.write_char('.')
    }
}

impl Drop for FormatterSink<'_, '_> {
    fn drop(&mut self) {
        println!("drop FormatterSink");
    }
}

struct Encoded<'a>(&'a str);

impl Display for Encoded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sink = FormatterSink { f };
        encode(self.0, &mut sink)
    }
}

// A writer that prints, and gives its `Formatter` to generic code and to a
// struct that holds it.
fn exclaim<W: Write>(w: &mut W, word: &str) -> fmt::Result {
    w.write_str(word)?;
    w.write_char('!')
}

struct Held<'a, 'b>(&'a mut fmt::Formatter<'b>);

// A `String`'s `&mut`, held beside it: a handle that reads its text.
struct Collected<'a>(&'a mut String);

impl Write for Collected<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        self.0.push_str(&self.0.len().to_string());
        Ok(())
    }
}

struct Loud(u8);

impl Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        println!("inside Loud {}", self.0);
        exclaim(f, "generic")?;
        let held = Held(f);
        held.0.write_str(" held")?;
        write!(held.0, " {}", self.0)?;
        let mut text = String::new();
        let mut collected = Collected(&mut text);
        collected.write_str("ab")?;
        exclaim(&mut collected, "cd")?;
        write!(f, " {text} {}", text.len())?;
        f.write_str(">")
    }
}

fn main() {
    println!("{:?}", Message("say \"hi\"".to_string()));
    println!("{}", Encoded("a b c"));
    println!("[{}] [{}]", Loud(1), Loud(2));
    let shown = format!("{:?} {} {}", Message("x".to_string()), Encoded("d e"), Loud(3));
    println!("{shown}");
}
