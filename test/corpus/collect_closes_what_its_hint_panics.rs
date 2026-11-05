// A `collect()` into a `Vec` of std's compiled adapter asks its source's
// `size_hint` for room first. Where that panics, `from_iter` drops what it
// took as it unwinds, so the source is dropped. Where it doesn't, the
// collect drops the source at its end, and nothing drops it again after
// (#660). (Which item std's `from_iter` takes before it asks, by
// `next`, is #724's.)
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Own {
    at: u32,
    hint_panics: bool,
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.at += 1;
        (self.at < 4).then_some(self.at)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.hint_panics {
            panic!("hint");
        }
        (0, Some(3))
    }
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("collected", || {
        let v: Vec<u32> = Own { at: 0, hint_panics: false }.fuse().collect();
        println!("collected {v:?}");
        v.len()
    });
    run("its hint panics", || {
        let v: Vec<u32> = Own { at: 0, hint_panics: true }.fuse().collect();
        v.len()
    });
    run("peeked, its hint panics", || {
        let v: Vec<u32> = Own { at: 0, hint_panics: true }.peekable().collect();
        v.len()
    });
}
