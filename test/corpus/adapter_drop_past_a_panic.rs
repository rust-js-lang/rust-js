// An adapter std compiles dropped part-way, whose first part's drop
// panics: what it holds after that part still dropped, as its drop glue
// goes on past a panicking field, a peekable's peeked item after its
// source (#677).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop");
        }
    }
}

fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut it = vec![D(1), D(2), D(3), D(4)].into_iter().peekable();
        println!("peeked {:?}", it.peek().map(|d| d.0));
        drop(it);
    }));
    println!("{}", r.is_err());
}
