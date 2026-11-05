// A `collect()` into a `Vec` of std's compiled adapter of what isn't
// `TrustedLen` takes its first item by `next`, then asks its source's
// `size_hint` for room (std's `SpecFromIterNested`); of none, it asks
// nothing and steps nothing again (#724). Where the hint panics,
// `from_iter` drops the item it took, then the source, as it unwinds.
// Where it doesn't, the collect drops the source at its end, and nothing
// drops it again after (#660).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own {
    at: u32,
    end: u32,
    hint_panics: bool,
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own at {}", self.at);
    }
}

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        println!("next at {}", self.at);
        self.at += 1;
        (self.at <= self.end).then(|| D(self.at))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.at);
        if self.hint_panics {
            panic!("hint");
        }
        (0, Some(3))
    }
}

fn own(end: u32, hint_panics: bool) -> Own {
    Own { at: 0, end, hint_panics }
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("collected", || {
        let v: Vec<D> = own(3, false).fuse().collect();
        println!("collected {}", v.len());
        v.len()
    });
    run("none", || {
        let v: Vec<D> = own(0, false).fuse().collect();
        v.len()
    });
    run("none, peeked", || {
        let v: Vec<D> = own(0, false).peekable().collect();
        v.len()
    });
    run("its hint panics", || {
        let v: Vec<D> = own(3, true).fuse().collect();
        v.len()
    });
    run("peeked, its hint panics", || {
        let v: Vec<D> = own(3, true).peekable().collect();
        v.len()
    });
}
