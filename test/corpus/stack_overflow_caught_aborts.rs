//@ run-abort
// A stack overflow of a recursion that holds nothing to drop, under
// `catch_unwind`: its catch, the one that sees it, takes it for Rust's
// abort, never a panic, and the program's guard outside isn't dropped
// (`$catchable`, #1193).
struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped");
    }
}

fn count(n: u64) -> u64 {
    if n == u64::MAX {
        return 0;
    }
    count(n + 1) + 1
}

fn main() {
    let _guard = Loud;
    let caught = std::panic::catch_unwind(|| count(0)).is_err();
    println!("caught {caught}");
}
