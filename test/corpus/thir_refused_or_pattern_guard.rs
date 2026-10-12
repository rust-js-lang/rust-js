// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
pub fn f(p: (i32, i32)) -> i32 { match p { (a, _) | (_, a) if a > 10 => a, _ => 0 } }
fn main() { for p in [(3, 42), (42, 3), (1, 2), (20, 30)] { println!("{}", f(p)); } }
