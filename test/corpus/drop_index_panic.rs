//@ run-fail: index out of bounds: the len is 3 but the index is 3
// An index out of bounds panics with what's in scope still to drop: its
// destructors run as the panic unwinds, as Rust's do (ADR 0098).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn nth(items: &[u32], at: usize) -> u32 {
    let _held = Noisy("held");
    let read = items[at];
    println!("read {read}");
    read
}

fn main() {
    let items = vec![1, 2, 3];
    println!("{}", nth(&items, 1));
    println!("{}", nth(&items, 3));
}
