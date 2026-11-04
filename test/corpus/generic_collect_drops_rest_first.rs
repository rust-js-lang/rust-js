// Generic code's collect into a `Vec` that a drop's panic stops: of an
// iterator no type it's given collects in place, a `map().fuse()`'s, a
// `peekable()`'s or a `chain()`'s, what's left of the source is dropped
// before the `Vec` collected so far, as std's fold unwinds inside it; of a
// `map()` alone, as its caller's type picks (#672): over a `Vec`'s
// `IntoIter`, which std collects in place, the `Vec` first; over the
// crate's own iterator, or a chain, what's left first; of an `enumerate()`,
// by whether the source's items are as aligned and no smaller; of a type
// parameter's items, by their caller's layout; passed on through generic
// code, and into a `Result`. And of a `filter()` whose discarded item's drop
// panics: in place, by its fold, what it collected leaks, as std's does
// (#681); otherwise it's dropped.
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};

static BOOM: AtomicU32 = AtomicU32::new(0);
static CALLS: AtomicU32 = AtomicU32::new(0);

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

/// Counts up to 5, and prints as it's dropped.
struct Own(u32);

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        (self.0 <= 5).then(|| D(self.0))
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own at {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn fused<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<_> = src.map(|x| D(x.0 + 100)).fuse().collect();
    v.len()
}

fn peeked<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<_> = src.map(|x| D(x.0 + 100)).peekable().collect();
    v.len()
}

fn chained<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<_> = src.map(|x| D(x.0 + 100)).chain(vec![D(6)]).collect();
    v.len()
}

fn mapped<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<_> = src.map(|x| D(x.0 + 100)).collect();
    v.len()
}

fn counted<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<_> = src.map(|x| D(x.0 + 100)).enumerate().collect();
    v.len()
}

fn kept<T, I: Iterator<Item = T>>(src: I) -> usize {
    CALLS.store(0, Ordering::Relaxed);
    let v: Vec<T> = src
        .filter(|_| {
            if CALLS.fetch_add(1, Ordering::Relaxed) == 2 {
                panic!("filter");
            }
            true
        })
        .collect();
    v.len()
}

fn passed<J: Iterator<Item = D>>(src: J) -> usize {
    mapped(src.map(|x| D(x.0 + 10)))
}

fn tried<I: Iterator<Item = D>>(src: I) -> Result<usize, String> {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<D> = src.map(|x| Ok::<D, String>(D(x.0 + 100))).collect::<Result<_, _>>()?;
    Ok(v.len())
}

fn discarded<I: Iterator<Item = D>>(src: I) -> usize {
    BOOM.store(3, Ordering::Relaxed);
    let v: Vec<D> = src.filter(|d| d.0 != 3).collect();
    v.len()
}

fn main() {
    println!("map.fuse");
    println!("{}", catch_unwind(|| fused(ds().into_iter())).is_err());
    println!("map.peekable");
    println!("{}", catch_unwind(|| peeked(ds().into_iter())).is_err());
    println!("map.chain");
    println!("{}", catch_unwind(|| chained(ds().into_iter())).is_err());
    println!("map of a Vec, in place");
    println!("{}", catch_unwind(|| mapped(ds().into_iter())).is_err());
    println!("map of the crate's own");
    println!("{}", catch_unwind(|| mapped(Own(0))).is_err());
    println!("map of a chain");
    println!(
        "{}",
        catch_unwind(|| mapped(ds().into_iter().chain(vec![D(6)]))).is_err()
    );
    println!("enumerate of a Vec's, bigger");
    println!("{}", catch_unwind(|| counted(ds().into_iter())).is_err());
    let pairs = || vec![(1u32, 0u32), (2, 0), (3, 0), (4, 0), (5, 0)].into_iter();
    println!("enumerate of pairs, in place");
    println!("{}", catch_unwind(|| counted(pairs().map(|(a, b)| D(a + b)))).is_err());
    println!("a type parameter's, in place");
    println!("{}", catch_unwind(|| kept(ds().into_iter())).is_err());
    println!("a type parameter's, of the crate's own");
    println!("{}", catch_unwind(|| kept(Own(0))).is_err());
    println!("passed on, in place");
    println!("{}", catch_unwind(|| passed(ds().into_iter())).is_err());
    println!("passed on, of the crate's own");
    println!("{}", catch_unwind(|| passed(Own(0))).is_err());
    println!("into a Result, in place");
    println!("{}", catch_unwind(|| tried(ds().into_iter()).is_ok()).is_err());
    println!("into a Result, of the crate's own");
    println!("{}", catch_unwind(|| tried(Own(0)).is_ok()).is_err());
    println!("a discard's drop, in place");
    println!("{}", catch_unwind(|| discarded(ds().into_iter())).is_err());
    println!("a discard's drop, of the crate's own");
    println!("{}", catch_unwind(|| discarded(Own(0))).is_err());
}
