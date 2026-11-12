// `fill_with`'s `*item = f()` writes what `f` made though the drop of the
// item it replaces panics, as an assignment does: the slot holds the new
// value as the panic unwinds, and the old one is never dropped again (#1598).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        if self.0 == 12 && !std::thread::panicking() {
            panic!("drop of 12");
        }
        println!("drop {}", self.0);
    }
}

fn numbers(items: &[Loud]) -> Vec<u32> {
    items.iter().map(|l| l.0).collect()
}

fn main() {
    let mut items = vec![Loud(11), Loud(12), Loud(13)];
    let mut made = 0;
    let caught = catch_unwind(AssertUnwindSafe(|| {
        items.fill_with(|| {
            made += 1;
            Loud(made)
        });
    }));
    println!("caught {}, made {made}", caught.is_err());
    println!("{:?}", numbers(&items));
    items[1..].fill_with(|| Loud(7));
    println!("{:?}", numbers(&items));
}
