// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
pub struct L<'a> { c: std::str::Chars<'a> }
pub fn f(l: &mut L) -> Option<char> { l.c.next() }
fn main() { let s = String::from("héllo"); let mut l = L { c: s.chars() }; for _ in 0..7 { print!("{:?} ", f(&mut l)); } println!(); }
