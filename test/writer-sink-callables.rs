// A writer given its `Formatter` through a value, a `fn` pointer it's
// given, one it names, and a boxed `dyn Fn` (#1652, ADR 1135): each a
// writer framed as the writer that calls it is, written a piece at a time
// into the writer it's shown into, as std writes it. A case of its own, as
// a `dyn` written frames every writer and would hide that framing.
use std::fmt::{self, Write};

// Marks each char it's given by its own `write_char`.
struct Chars(Vec<String>);

impl Write for Chars {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push(format!("<{c}>"));
        Ok(())
    }
}

type Writing = fn(&mut fmt::Formatter<'_>, i32) -> fmt::Result;

fn bracketed(f: &mut fmt::Formatter<'_>, n: i32) -> fmt::Result {
    write!(f, "[{n}]")
}

// A `fn` pointer it's given, and one it names.
struct Via(Writing, i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f, self.1)?;
        let named: Writing = bracketed;
        named(f, -self.1)
    }
}

// A boxed closure it's given.
struct Boxed(Box<dyn Fn(&mut fmt::Formatter<'_>) -> fmt::Result>);

impl fmt::Display for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)?;
        f.write_char('!')
    }
}

fn main() {
    let k = 4;
    let via = Via(bracketed, -3);
    let boxed = Boxed(Box::new(move |f| write!(f, "<{}>", -k)));
    let mut chars = Chars(Vec::new());
    write!(chars, "{}{}", via, boxed).unwrap();
    println!("{:?} {} {}", chars.0, via, boxed);
}
