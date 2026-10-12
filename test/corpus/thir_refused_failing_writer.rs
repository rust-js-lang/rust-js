// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
//@ thir-compile-fail: a `fmt::Error` of a writer of the crate's
use std::fmt::{self, Write};
pub struct Full(pub usize);
impl Write for Full { fn write_str(&mut self, s: &str) -> fmt::Result { if s.len() > self.0 { return Err(fmt::Error); } self.0 -= s.len(); Ok(()) } }
fn main() { let mut f = Full(5); println!("{:?} {}", f.write_str("abc"), f.0); println!("{:?} {}", f.write_str("abc"), f.0); println!("{:?} {}", write!(f, "{}", 1), f.0); }
