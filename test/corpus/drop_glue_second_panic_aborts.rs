//@ run-abort
// Drop glue whose part panics drops the parts after it as that panic
// unwinds, each in turn; one of those that panics too aborts there, and the
// parts after it are never dropped (#865): `drop 1`, part 2 panics, part 3
// panics, then the abort, no `drop 4`. Its `catch` once ran each later part
// in a `try` of its own, so `drop 4` printed before the abort.
struct Part(u32);

impl Drop for Part {
    fn drop(&mut self) {
        if self.0 == 2 || self.0 == 3 {
            panic!("part {} panics", self.0);
        }
        println!("drop {}", self.0);
    }
}

struct Four {
    a: Part,
    b: Part,
    c: Part,
    d: Part,
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        let _four = Four {
            a: Part(1),
            b: Part(2),
            c: Part(3),
            d: Part(4),
        };
    });
    println!("never {}", r.is_err());
}
