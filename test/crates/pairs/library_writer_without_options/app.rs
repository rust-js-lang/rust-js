//! An app whose writers take options, `{:>12}` and `{:<10}`, giving its
//! formatter to the library's plain writers, `self.0.fmt(f)`: they pad
//! nothing, as Rust's `write!` of them doesn't, and the app's own `{:>12}`
//! of a library value pads it, as `Display` of the whole is padded (#1376:
//! decision coverage found no direct case of a plain library writer).
use dep::{Meters, Unit};
use std::fmt;

struct Report(Meters, Unit);

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)?;
        write!(f, " ")?;
        self.1.fmt(f)
    }
}

struct Padded(String);

impl fmt::Display for Padded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.0)
    }
}

pub fn main() {
    let report = Report(Meters(2.5), Unit::Metric);
    println!("[{report:>12}]");
    println!("[{:<10}]", Report(Meters(-1.0), Unit::Imperial));
    println!("[{:>12}]", Padded(Meters(3.0).to_string()));
    println!("[{:>12}] [{}]", Meters(4.25).to_string(), Unit::Imperial);
}
