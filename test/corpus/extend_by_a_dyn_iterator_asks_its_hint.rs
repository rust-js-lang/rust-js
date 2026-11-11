// `extend` of a `Vec` by an iterator whose type doesn't say whether it
// has an end, a `Box<dyn Iterator>`, asks its hint as std's
// `extend_desugared` does, whenever the `Vec` is full: of the crate's own
// iterator behind it, its `size_hint` runs there, a `map` of it too; of
// one that has none, its lower bound `usize::MAX`, `capacity overflow`
// once it's full (#1377, as #1359's collect).
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

/// The crate's own iterator, whose hint says when it's asked.
struct Counted(u32, u32);

impl Iterator for Counted {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= self.1).then_some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint asked at {}", self.0);
        (0, None)
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "another payload".to_string())
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(made) => println!("{what}: {made:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn main() {
    check("extend by a boxed own", || {
        let mut v = vec![0];
        let items: Box<dyn Iterator<Item = u32>> = Box::new(Counted(0, 6));
        v.extend(items);
        v.len()
    });
    check("extend by a boxed endless", || {
        let mut v: Vec<u8> = Vec::new();
        let items: Box<dyn Iterator<Item = u8>> = Box::new(0u8..);
        v.extend(items);
        v.len()
    });
    check("extend by a mapped boxed own", || {
        let mut v: Vec<u32> = Vec::new();
        let items: Box<dyn Iterator<Item = u32>> = Box::new(Counted(0, 5));
        v.extend(items.map(|n| n * 10));
        v
    });
}
