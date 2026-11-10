//@ run-abort
// A stack overflow no `catch_unwind` is around: Rust's abort, exit 134,
// before any frame's value drops (#1193).
struct Loud(u64);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn deep(n: u64, held: &mut Vec<u64>) -> u64 {
    let _l = Loud(n);
    held.push(n);
    if n == u64::MAX {
        return 0;
    }
    deep(n + 1, held) + 1
}

fn main() {
    let _guard = Loud(7);
    let mut held = Vec::new();
    println!("{}", deep(0, &mut held));
}
