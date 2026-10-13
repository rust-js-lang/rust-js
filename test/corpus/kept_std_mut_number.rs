// A `&mut` to a map's number, `get_mut`'s, bound to a variable, kept past
// the `match` and written through: the map's value changes.
use std::collections::HashMap;
pub fn f(m: &mut HashMap<u32, i32>) { let k; match m.get_mut(&1) { Some(v) => k = v, None => return } *k += 1; }
fn main() { let mut m = HashMap::new(); f(&mut m); println!("{:?}", m.get(&1)); m.insert(1, 5); f(&mut m); f(&mut m); println!("{:?}", m.get(&1)); }
