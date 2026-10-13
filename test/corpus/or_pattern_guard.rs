// A guard of a `|` pattern binding a name at another place in each
// alternative is tried with each in turn: `(3, 42)` takes the arm, `a` 42
// (rustc's issue-70413).
pub fn f(p: (i32, i32)) -> i32 { match p { (a, _) | (_, a) if a > 10 => a, _ => 0 } }
fn main() { for p in [(3, 42), (42, 3), (1, 2), (20, 30)] { println!("{}", f(p)); } }
