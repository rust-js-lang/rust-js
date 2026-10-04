//@ run-fail: assertion failed: self.is_char_boundary(idx)
// `insert` inside a character panics, as Rust's does.

fn main() {
    let mut s = String::from("héllo");
    s.insert(1, 'x');
    s.insert(3, 'y');
    println!("{s}");
}
