// A `Vec`'s iterator behind a `Box<dyn Iterator>`, stepped by `step_by`,
// which skips by `nth`: the box forwards `nth` to the `Vec`'s, which drops
// what it passes as one slice, so where one of those drops panics, the
// rest of that slice still drops before the fold's accumulator does
// (#1690).
use std::panic::{self, AssertUnwindSafe};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop 3");
        }
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn boxed(items: Vec<D>) -> Box<dyn Iterator<Item = D>> {
    Box::new(items.into_iter())
}

fn main() {
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        let items = vec![D(1), D(2), D(3), D(4), D(5)];
        let acc = boxed(items).step_by(2).step_by(2).fold(Acc(0), |mut a, x| {
            a.0 += x.0;
            a
        });
        println!("acc {}", acc.0);
    }));
    println!("caught {}", caught.is_err());
}
