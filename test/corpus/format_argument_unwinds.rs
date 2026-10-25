// A `format_args!` argument that may panic, read out of the order it's
// made in, a named one, `{total}`, shown first though it's made last, is
// held where it's made, `const _arg = first(..)`: in the `try` of what's
// live there, so the guard is dropped as the panic unwinds, as Rust drops
// it (the guide's semantics: destructors).
//@ run-fail: index out of bounds: the len is 0 but the index is 0
struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn first(items: &[u32]) -> u32 {
    items[0]
}

fn main() {
    let total = 5;
    let _guard = Guard("outer");
    println!("{total} {}", first(&[1]));
    println!("{total} {} {}", first(&[2]), first(&[]));
}
