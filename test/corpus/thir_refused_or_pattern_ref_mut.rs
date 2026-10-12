// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
pub fn f(p: &mut (i32, i32)) { if let (0, x) | (x, 0) = p { *x += 1; } }
fn main() { for mut p in [(0, 5), (7, 0), (0, 0), (3, 4)] { f(&mut p); println!("{:?}", p); } }
