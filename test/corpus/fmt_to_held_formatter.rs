// A `fmt` trait's method called as a function on a `Formatter` held where
// it isn't its writer's (ADR 0461), `Display::fmt(&x, self.0)`: what it
// writes, appended to the handle's text, as anyhow's `Quoted` writer
// escapes what it's given into the `Formatter` it holds.
use std::fmt::{self, Debug, Display, Write};

// anyhow's `Quoted`: a writer over a `Formatter` it holds.
struct Quoted<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl Write for Quoted<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Display::fmt(&s.escape_debug(), self.0)
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

// A holder that shows a value both ways into the `Formatter` it holds.
struct Tagged<'a, 'b> {
    tag: &'static str,
    f: &'a mut fmt::Formatter<'b>,
}

impl Tagged<'_, '_> {
    fn show<T: Display + Debug>(&mut self, value: &T) -> fmt::Result {
        self.f.write_str(self.tag)?;
        Display::fmt(value, self.f)?;
        self.f.write_str(" / ")?;
        Debug::fmt(value, self.f)
    }
}

struct Pair(u32, &'static str);

impl Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Tagged { tag: "pair ", f }.show(&self.0)?;
        f.write_str(", ")?;
        Tagged { tag: "", f }.show(&self.1)
    }
}

// A derived `Debug` written to a held `Formatter`: plain, `{ x: 1 }` on one
// line, as the handle keeps no `{:#?}`.
#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

struct At<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl At<'_, '_> {
    fn point(&mut self, point: &Point) -> fmt::Result {
        self.0.write_str("at ")?;
        Debug::fmt(point, self.0)
    }
}

struct Spot(Point);

impl Display for Spot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        At(f).point(&self.0)
    }
}

fn main() {
    println!("{:?}", Message(String::from("say \"hi\"\n")));
    println!("{}", Pair(7, "x"));
    println!("{}", Spot(Point { x: 1, y: -2 }));
    // Pretty where it's asked of the `Formatter` itself, not of the handle.
    println!("{:#?}", Point { x: 3, y: 4 });
}
