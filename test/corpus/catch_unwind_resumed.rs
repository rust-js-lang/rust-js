//@ run-fail: resumed 3
// A panic caught and resumed goes on as the panic it was (ADR 0660), its
// message its payload's, what was dropped as it unwound dropped once.
use std::panic;

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _outer = Noisy(0);
    let result = panic::catch_unwind(|| {
        let _inner = Noisy(1);
        panic!("resumed {}", 3);
    });
    println!("caught");
    if let Err(payload) = result {
        panic::resume_unwind(payload);
    }
}
