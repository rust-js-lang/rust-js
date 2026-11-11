// `collect()` into a `Vec` of an iterator whose type doesn't say whether
// it has an end, a `Box<dyn Iterator>` or a `&mut dyn Iterator`, asks its
// hint as std's `SpecFromIterNested` does, after its first item: of one
// that has none, a `RangeFrom`'s or a `repeat`'s, its lower bound is
// `usize::MAX`, so `capacity overflow` panics after one item, the item
// taken dropped; and of the crate's own iterator, its `size_hint` runs,
// once, where std's asks it (the user's decision, #1359).
use std::any::Any;
use std::iter::repeat;
use std::panic::{self, AssertUnwindSafe};

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

/// The crate's own iterator, whose hint says when it's asked.
struct Counted(u32);

impl Iterator for Counted {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 4).then_some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint asked at {}", self.0);
        (0, None)
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(made) => println!("{what}: {made:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn collected<T>(items: Box<dyn Iterator<Item = T>>) -> Vec<T> {
    items.collect()
}

fn main() {
    check("boxed range", || collected(Box::new(0u8..)).len());
    check("boxed repeat", || collected(Box::new(repeat(7u32))).len());
    check("boxed endless noisy", || collected(Box::new((0..).map(Noisy))).len());
    check("boxed finite", || collected(Box::new([1, 2, 3].into_iter())));
    check("boxed empty", || collected(Box::new(std::iter::empty::<u8>())));
    check("boxed own", || collected(Box::new(Counted(0))));
    check("mapped boxed own", || {
        let items: Box<dyn Iterator<Item = u32>> = Box::new(Counted(0));
        items.map(|n| n * 10).collect::<Vec<_>>()
    });
    check("lent", || {
        let mut items = Counted(0);
        let lent: &mut dyn Iterator<Item = u32> = &mut items;
        lent.collect::<Vec<_>>()
    });
    check("lent range", || {
        let mut endless = 0u16..;
        let lent: &mut dyn Iterator<Item = u16> = &mut endless;
        lent.collect::<Vec<_>>().len()
    });
}
