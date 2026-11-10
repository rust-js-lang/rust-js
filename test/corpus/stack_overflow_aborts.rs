//@ run-abort
// A stack overflow is Rust's abort, exit 134, std's lines to stderr, and
// no destructor run, though `catch_unwind` is around it and each frame
// holds a value whose drop prints: the engine's overflow is told apart
// (`$isStackOverflow`), and each scope whose drops a program sees aborts
// before them, `catch (error) { throw $unwindsOn(error); }` (#1193).
struct Loud(u64);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn deep(n: u64) -> u64 {
    let _l = Loud(n);
    if n == u64::MAX {
        return 0;
    }
    deep(n + 1) + 1
}

fn main() {
    let _guard = Loud(999_999);
    println!("before");
    let r = std::panic::catch_unwind(|| deep(0));
    println!("caught {}", r.is_err());
}
