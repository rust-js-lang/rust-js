// A `ref mut` bound at another place in each alternative of a `|` pattern:
// what's written through is the place that matched.
pub fn f(p: &mut (i32, i32)) { if let (0, x) | (x, 0) = p { *x += 1; } }
fn main() { for mut p in [(0, 5), (7, 0), (0, 0), (3, 4)] { f(&mut p); println!("{:?}", p); } }
