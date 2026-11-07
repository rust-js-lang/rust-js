//@ run-abort
// An enum with a `Drop` of its own, whose `drop` panics: as that panic
// unwinds, rustc's drop glue drops the variant's fields one after another,
// and one of them that panics too aborts there (#887). So `drop 1`, `drop 2`
// panics, then the abort, and there's no `drop 3`. Each variant's fields
// were written by hand with a `try` of their own inside the cleanup, so
// `drop 3` printed before the abort.
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("loud {} panics", self.0);
        }
    }
}

enum Shape {
    Three(Loud, Loud, Loud),
    Empty,
}

impl Drop for Shape {
    fn drop(&mut self) {
        println!("shape");
        panic!("shape panics");
    }
}

fn main() {
    let shape = Shape::Three(Loud(1), Loud(2), Loud(3));
    if let Shape::Empty = shape {
        println!("empty");
    }
    let r = std::panic::catch_unwind(move || drop(shape));
    println!("never {}", r.is_err());
}
