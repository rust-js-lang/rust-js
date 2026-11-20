// A `Box` of std's `map` over a `Vec`'s items with a destructor, its closure
// holding one too, folded by a consumer that panics: std's `Box<I>` folds by
// the trait's default over its `next`, so as the panic unwinds it drops the
// box whole, the items left first, then the closure; std's `Map::fold`,
// unboxed, gives its closure to the items' own `fold`, which drops it
// first. Only the order of the drops tells the two apart, so the box hides
// what it holds's steps however little of them reaches the crate's code
// (`box_hides_its_steps`, #2185).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let guard = D(100);
    let boxed = Box::new(vec![D(1), D(2), D(3)].into_iter().map(move |d| {
        let _ = &guard;
        d.0
    }));
    let folded = catch_unwind(AssertUnwindSafe(|| {
        boxed.fold(0, |a, x| {
            if x == 2 {
                panic!("stop");
            }
            a + x
        })
    }));
    println!("folded {}", folded.is_ok());
    let guard = D(200);
    let unboxed = vec![D(4), D(5), D(6)].into_iter().map(move |d| {
        let _ = &guard;
        d.0
    });
    let folded = catch_unwind(AssertUnwindSafe(|| {
        unboxed.fold(0, |a, x| {
            if x == 5 {
                panic!("stop");
            }
            a + x
        })
    }));
    println!("folded {}", folded.is_ok());
}
