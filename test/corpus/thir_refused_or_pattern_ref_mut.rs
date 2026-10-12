// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
//@ thir-compile-fail: a `ref mut` bound at another place in each alternative of a `|` pattern
pub fn f(p: &mut (i32, i32)) { if let (0, x) | (x, 0) = p { *x += 1; } }
fn main() { for mut p in [(0, 5), (7, 0), (0, 0), (3, 4)] { f(&mut p); println!("{:?}", p); } }
