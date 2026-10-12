// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
pub struct D(u32);
impl Drop for D { fn drop(&mut self) { println!("drop {}", self.0); } }
impl D { pub fn n(&self) -> u32 { println!("n {}", self.0); 1 } }
pub fn f(b: bool) -> u32 { if D(1).n() == 1 && let true = b { println!("then"); 1 } else { println!("else"); 0 } }
fn main() { println!("{}", f(true)); println!("{}", f(false)); }
