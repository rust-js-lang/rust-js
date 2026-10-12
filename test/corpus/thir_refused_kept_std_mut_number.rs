// What THIR refuses, MIR lowers as Rust runs it (ADR 0364).
//@ thir-compile-fail: a `&mut i32` from a std call used as a value
use std::collections::HashMap;
pub fn f(m: &mut HashMap<u32, i32>) { let k; match m.get_mut(&1) { Some(v) => k = v, None => return } *k += 1; }
fn main() { let mut m = HashMap::new(); f(&mut m); println!("{:?}", m.get(&1)); m.insert(1, 5); f(&mut m); f(&mut m); println!("{:?}", m.get(&1)); }
