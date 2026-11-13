//! A library's writer that calls a callable it's given with its
//! `Formatter`, a boxed closure and a `fn` pointer, which its consumer
//! makes: the consumer frames each, so each writes into the sink the
//! library's writer arms for it (#1736, ADR 1135).
use std::fmt::{self, Write};

pub struct Chars(pub Vec<String>);

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

pub type Writing = fn(&mut fmt::Formatter<'_>, i32) -> fmt::Result;

pub struct Via(pub Writing, pub i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f, self.1)
    }
}

pub struct Boxed(pub Box<dyn Fn(&mut fmt::Formatter<'_>) -> fmt::Result>);

impl fmt::Display for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)?;
        f.write_char('!')
    }
}
