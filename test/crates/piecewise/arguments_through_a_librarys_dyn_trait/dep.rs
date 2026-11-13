//! A library's trait whose impl writes the `Arguments` it's given into a
//! writer, which its contract says of the trait's method, not an item: a
//! consumer's `format_args!` given it through a `dyn` is the function that
//! writes into the sink it claims, each piece the writer's (#1735).
use std::fmt::{self, Write};

pub struct Log(pub Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

pub struct Point(pub i32, pub i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub trait Sink {
    fn put(&mut self, args: fmt::Arguments<'_>) -> fmt::Result;
    fn text(&self) -> String;
}

impl Sink for Log {
    fn put(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.write_fmt(args)
    }

    fn text(&self) -> String {
        format!("{:?}", self.0)
    }
}
