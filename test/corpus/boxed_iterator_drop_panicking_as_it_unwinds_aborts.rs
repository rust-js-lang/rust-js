//@ run-abort
// A `Box<dyn Iterator>` counted, whose third item's drop panics: std's
// default `count` drops each item it's given, so that panic unwinds out of
// it, and the box, dropped as `count`'s `self` ends, drops what it still
// holds as one slice; the next of those drops panics too, while the first
// unwinds, which aborts the process, though a `catch_unwind` would have
// caught the first (#1690).
use std::sync::atomic::{AtomicU32, Ordering};

static DROPS: AtomicU32 = AtomicU32::new(0);

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 >= 3 && DROPS.fetch_add(1, Ordering::Relaxed) < 2 {
            panic!("drop {}", self.0);
        }
    }
}

fn boxed(items: Vec<D>) -> Box<dyn Iterator<Item = D>> {
    Box::new(items.into_iter())
}

fn main() {
    let caught = std::panic::catch_unwind(|| {
        let n = boxed(vec![D(1), D(2), D(3), D(4), D(5)]).count();
        println!("count {n}");
    });
    println!("caught {}", caught.is_err());
}
