// An `Arguments` shown by `Display::fmt(&args, f)` into a writer is written
// a piece at a time, as std's `fmt::write` of it writes it, wherever it was
// made: in the body, strum's `Display::fmt(&format_args!(..), f)`; kept in
// a field, an `Option` or a closure's capture; returned by a function; or
// lent by a `&Arguments` parameter; or a constant. One a function returned
// is written so by a `write_fmt`, and by a function that gives it to one,
// too (#1652, step 8 of ADR 1135).
use std::fmt::{self, Display, Write};

// Records each piece it's given.
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

// Fails at its `n`th call.
struct Failing(u32, u32, String);

impl Write for Failing {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += 1;
        if self.0 == self.1 {
            return Err(fmt::Error);
        }
        self.2.push_str(s);
        Ok(())
    }
}

// Made in the body, as strum's derived `Display` is.
enum Color {
    Red,
    Named(&'static str, u8),
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Red => Display::fmt(&format_args!("red"), f),
            Color::Named(name, n) => Display::fmt(&format_args!("{name}#{n:>3}"), f),
        }
    }
}

// Kept in a field.
struct Line<'a>(fmt::Arguments<'a>);

impl Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        Display::fmt(&self.0, f)?;
        f.write_str("]")
    }
}

// Kept in an `Option`.
struct Maybe<'a>(Option<fmt::Arguments<'a>>);

impl Display for Maybe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(args) => Display::fmt(args, f),
            None => f.write_str("-"),
        }
    }
}

// Kept in a `Box`, and written from a field by `write_fmt`.
struct Boxed<'a>(Box<fmt::Arguments<'a>>, fmt::Arguments<'a>);

impl Display for Boxed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.0, f)?;
        f.write_fmt(self.1)
    }
}

fn given(args: fmt::Arguments<'_>) -> fmt::Arguments<'_> {
    args
}

// Lent by a parameter.
fn shown(args: &fmt::Arguments<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("<")?;
    Display::fmt(args, f)?;
    f.write_str(">")
}

struct Lent(i32);

impl Display for Lent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        shown(&format_args!("{}:{}", self.0, -self.0), f)
    }
}

// Captured by a closure.
struct Captured(u8);

impl Display for Captured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = format_args!("{}{}", self.0, 'c');
        let show = |f: &mut fmt::Formatter<'_>| Display::fmt(&args, f);
        show(f)?;
        show(f)
    }
}

// A constant's, written and shown.
const GREETING: fmt::Arguments<'static> = format_args!("hello");

struct Greeted;

impl Display for Greeted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(GREETING)?;
        Display::fmt(&GREETING, f)
    }
}

// Returned by a function, then written.
struct Relayed;

impl Display for Relayed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(given(format_args!("{}-{}", 1, "r")))?;
        Display::fmt(&given(format_args!("{}+{}", 2, 's')), f)
    }
}

fn log_to(w: &mut Log, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn logged<T: Display>(value: &T) -> String {
    let mut log = Log(Vec::new());
    write!(log, "{value}").unwrap();
    format!("{:?}", log.0)
}

fn failing<T: Display>(value: &T, at: u32) -> String {
    let mut w = Failing(0, at, String::new());
    let r = write!(w, "{value}");
    format!("{:?} {:?}", r, w.2)
}

fn main() {
    let colors = [Color::Red, Color::Named("teal", 7)];
    for color in &colors {
        println!("{color} {:>10}| {}", color, logged(color));
    }
    println!("{}", failing(&colors[1], 2));

    let line = Line(format_args!("{}|{:^5}|{}", 1, 'x', "end"));
    println!("{line} {}", logged(&line));
    for at in 1..=6 {
        println!("{}", failing(&line, at));
    }

    let some = Maybe(Some(format_args!("{}{}", 'a', 2)));
    println!("{some} {} {}", logged(&some), logged(&Maybe(None)));
    println!("{}", failing(&some, 2));

    let boxed = Boxed(Box::new(format_args!("boxed")), format_args!("{}{}", 'w', 2));
    println!("{boxed} {}", logged(&boxed));
    println!("{}", failing(&boxed, 3));

    println!("{} {}", Lent(4), logged(&Lent(4)));
    println!("{}", failing(&Lent(4), 3));
    println!("{} {}", Captured(9), logged(&Captured(9)));
    println!("{}", failing(&Captured(9), 3));
    println!("{} {}", Relayed, logged(&Relayed));
    for at in [2, 5] {
        println!("{}", failing(&Relayed, at));
    }

    println!("{} {} {GREETING}", Greeted, logged(&Greeted));
    println!("{}", failing(&Greeted, 2));

    let mut log = Log(Vec::new());
    log_to(&mut log, given(format_args!("{}|{:>3}", 1, 'k'))).unwrap();
    println!("{:?}", log.0);
}
