//@ run-abort
// An iterator's step that panics drops what the iterator holds as it
// unwinds: one of those whose drop panics then aborts (#847), as Rust does.
use std::panic;

struct Bomb(&'static str);

impl Drop for Bomb {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == "bomb" {
            panic!("bomb went off");
        }
    }
}

fn main() {
    let items = vec![Bomb("a"), Bomb("b"), Bomb("go"), Bomb("bomb"), Bomb("z")];
    let r = panic::catch_unwind(move || {
        let taken: Vec<Bomb> = items
            .into_iter()
            .map(|b| {
                if b.0 == "go" {
                    panic!("step panicked");
                }
                b
            })
            .collect();
        taken.len()
    });
    println!("never {}", r.is_ok());
}
