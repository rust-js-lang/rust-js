// A key with a destructor, in `max_by_key`, `min_by_key` and
// `is_sorted_by_key`, drops where std drops it (#776). `max_by_key` pairs
// each item with its key, `(key, item)`, and keeps the best pair, dropping
// each pair it passes over, key first, then item. The kept pair's key
// drops before its item is given back. `is_sorted_by_key` keeps the last
// key and drops each one it replaces. Where two keys are out of order, it
// drops the later key, then the last one, then what's left. A slice's
// `is_sorted_by_key` is its iterator's, and drops its keys so too (#783).
// This is run over the crate's iterator, a slice's and a `Vec`'s items
// with a destructor of their own, and as a key's function, its comparison
// and a key's drop panic part way, and where what `min_by` and `max_by`
// pass over panics as it drops (#785).
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, Debug)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
        if self.0 == 13 {
            panic!("key drop");
        }
    }
}

impl PartialOrd for K {
    fn partial_cmp(&self, other: &K) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for K {
    fn cmp(&self, other: &K) -> Ordering {
        assert!(self.0 != 99 && other.0 != 99, "compare");
        self.0.cmp(&other.0)
    }
}

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop item {}", self.0);
    }
}

/// The crate's iterator, `from + 1` up to `to`.
struct Up(u32, u32);

impl Iterator for Up {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

fn ds(keys: &[u32]) -> Vec<D> {
    keys.iter().map(|&k| D(k)).collect()
}

fn run(name: &str, f: impl FnOnce()) {
    let caught = catch_unwind(AssertUnwindSafe(f));
    println!("{name} panicked {}", caught.is_err());
}

fn main() {
    println!("{:?}", Up(0, 4).max_by_key(|x| K(x % 3)));
    println!("{:?}", Up(0, 4).min_by_key(|x| K(x % 3)));
    println!("{:?}", Up(0, 0).max_by_key(|x| K(*x)));
    println!("{:?}", Up(0, 1).min_by_key(|x| K(*x)));
    println!("{:?}", [4u32, 9, 2].iter().max_by_key(|x| K(**x)));
    println!("{:?}", ds(&[1, 5, 2]).into_iter().max_by_key(|d| K(d.0)));
    println!("{:?}", ds(&[1, 5, 2]).into_iter().min_by_key(|d| K(d.0)));

    println!("{}", Up(0, 4).is_sorted_by_key(|x| K(x % 3)));
    println!("{}", Up(0, 3).is_sorted_by_key(K));
    println!("{}", Up(0, 0).is_sorted_by_key(K));
    println!("{}", [3u32, 1, 2].iter().is_sorted_by_key(|x| K(*x)));
    println!("{}", ds(&[2, 1, 3]).into_iter().is_sorted_by_key(|d| K(d.0)));
    println!("{}", ds(&[1, 2, 3]).into_iter().is_sorted_by_key(|d| K(d.0)));
    // A slice's, `self.iter().is_sorted_by_key(f)` (#783).
    let v = [1u32, 3, 2, 4];
    println!("{}", v.is_sorted_by_key(|x| K(*x)));
    println!("{}", v[..2].is_sorted_by_key(|x| K(*x)));
    println!("{}", v[..0].is_sorted_by_key(|x| K(*x)));

    run("max_by_key's key", || {
        let best = Up(0, 5).max_by_key(|x| {
            assert!(*x != 3, "key");
            K(*x)
        });
        println!("{best:?}");
    });
    run("max_by_key's compare", || {
        let best = Up(0, 3).max_by_key(|x| K(if *x == 2 { 99 } else { *x }));
        println!("{best:?}");
    });
    run("min_by_key's key drop", || {
        let best = ds(&[12, 13, 14]).into_iter().min_by_key(|d| K(d.0));
        println!("{best:?}");
    });
    run("max_by_key's kept key drop", || {
        let best = ds(&[12, 13]).into_iter().max_by_key(|d| K(d.0));
        println!("{best:?}");
    });
    run("is_sorted_by_key's key", || {
        let sorted = Up(0, 5).is_sorted_by_key(|x| {
            assert!(x != 3, "key");
            K(x)
        });
        println!("{sorted}");
    });
    run("is_sorted_by_key's compare", || {
        let sorted = Up(0, 3).is_sorted_by_key(|x| K(if x == 2 { 99 } else { x }));
        println!("{sorted}");
    });
    run("is_sorted_by_key's replaced key", || {
        let sorted = ds(&[12, 13, 14]).into_iter().is_sorted_by_key(|d| K(d.0));
        println!("{sorted}");
    });
    // What `min_by` and `max_by` keep is leaked where the one they pass
    // over panics as it drops, as rustc leaks what a function returns as a
    // later drop unwinds (#785).
    run("min_by's passed-over drop", || {
        let least = vec![K(12), K(13), K(14)].into_iter().min_by(|a, b| a.0.cmp(&b.0));
        println!("{least:?}");
    });
    run("max_by's passed-over drop", || {
        let most = vec![K(13), K(14), K(1)].into_iter().max_by(|a, b| a.0.cmp(&b.0));
        println!("{most:?}");
    });
    run("a slice's is_sorted_by_key's compare", || {
        let sorted = [1u32, 99, 2].is_sorted_by_key(|x| K(*x));
        println!("{sorted}");
    });
    run("a slice's is_sorted_by_key's replaced key", || {
        let sorted = [12u32, 13, 14].is_sorted_by_key(|x| K(*x));
        println!("{sorted}");
    });
}
