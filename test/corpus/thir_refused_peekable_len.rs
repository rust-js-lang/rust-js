// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
pub fn f(v: &[u32]) -> usize { let mut p = v.iter().peekable(); p.peek(); p.len() }
fn g(v: &[u32]) -> (usize, Option<u32>, usize) { let mut p = v.iter().peekable(); let a = p.next().copied(); p.peek(); (p.len(), a, p.count()) }
fn main() { println!("{} {} {:?} {:?}", f(&[1, 2, 3]), f(&[]), g(&[1, 2, 3]), g(&[])); }
