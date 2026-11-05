// `last()` by its fold, where a drop panics after the fold has what it
// kept in hand: a `filter()`'s discarded item, whose drop runs once its
// closure has given back the item `last` kept; a `take()`'s source,
// dropped once it has folded all it takes; and a `zip()`'s side's rest,
// dropped once its default fold has stepped it to its end. Rustc doesn't
// drop what a function gives back as a later drop unwinds, so the item
// kept is leaked there, not dropped (#648). And a `flatten()`'s `last`, where the drop of
// the one kept before panics as an inner iterator's `last` replaces it:
// it isn't dropped again as the panic unwinds, and the new one is leaked.
// Where a stage panics, or a step, the one kept is the fold's to drop, and
// is dropped, before what's left.
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, Ordering};

/// The item whose drop panics, once.
static BOOM: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.load(Ordering::Relaxed) {
            BOOM.store(0, Ordering::Relaxed);
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

/// A crate's iterator that panics as it's asked its `n`th item.
struct Panics {
    at: u32,
    n: u32,
}

impl Iterator for Panics {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.at += 1;
        if self.at == self.n {
            panic!("next");
        }
        (self.at < 6).then(|| D(self.at + 10))
    }
}

fn run(name: &str, f: impl FnOnce() -> Option<u32>) {
    println!("{name}");
    BOOM.store(3, Ordering::Relaxed);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("filtered", || {
        ds().into_iter()
            .zip(10..13u32)
            .filter(|(d, n)| (d.0 * 1000 + n) % 2 == 1)
            .last()
            .map(|(d, _)| d.0 + 0)
    });
    run("filtered, in place", || {
        ds().into_iter().filter(|d| d.0 % 2 == 0).last().map(|d| d.0 + 0)
    });
    run("taken from a flatten", || {
        ds().into_iter()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .take(3)
            .last()
            .map(|d| d.0 + 0)
    });
    run("zipped with a flatten", || {
        ds().into_iter()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .zip(10..13u32)
            .last()
            .map(|(d, _)| d.0 + 0)
    });
    run("flattened, cloned", || {
        let items = ds();
        items
            .iter()
            .cloned()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .last()
            .map(|d| d.0 + 0)
    });
    run("a stage panics", || {
        vec![D(1), D(2), D(4), D(5)]
            .into_iter()
            .filter(|d| {
                if d.0 == 4 {
                    panic!("stage");
                }
                true
            })
            .last()
            .map(|d| d.0 + 0)
    });
    run("a step panics", || {
        Panics { at: 0, n: 4 }.filter(|d| d.0 % 2 == 0).last().map(|d| d.0 + 0)
    });
}
