// Drop glue of many parts whose drops may panic (#865): a tuple's, a
// struct's and a generic tuple's, of up to 20 parts. Each part drops in
// turn; where part k panics, the rest still drop as it unwinds, then the
// panic goes on, for each k. The JS grows as the square of the parts, where
// each part's `catch` once copied all of what dropped after it, twice for
// each part: num-traits' 20-tuple `Bounded` was 240 MB of JS.
use std::cell::Cell;
use std::panic;

thread_local! {
    static TARGET: Cell<u32> = const { Cell::new(0) };
}

struct Part(u32);

impl Drop for Part {
    fn drop(&mut self) {
        if TARGET.get() == self.0 {
            panic!("part {} panics", self.0);
        }
        println!("drop {}", self.0);
    }
}

struct Four {
    a: Part,
    b: Part,
    c: Part,
    d: Part,
}

fn four() -> Four {
    Four {
        a: Part(1),
        b: Part(2),
        c: Part(3),
        d: Part(4),
    }
}

type Twenty<T> = (T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T);

fn twenty() -> Twenty<Part> {
    (
        Part(1),
        Part(2),
        Part(3),
        Part(4),
        Part(5),
        Part(6),
        Part(7),
        Part(8),
        Part(9),
        Part(10),
        Part(11),
        Part(12),
        Part(13),
        Part(14),
        Part(15),
        Part(16),
        Part(17),
        Part(18),
        Part(19),
        Part(20),
    )
}

/// A generic one's, given its parts' drop.
fn held<T>(parts: Twenty<T>, last: T) -> usize {
    let _parts = parts;
    let _last = last;
    std::mem::size_of::<T>()
}

fn main() {
    for target in 0..=4 {
        TARGET.set(target);
        let r = panic::catch_unwind(|| {
            let _tuple = (Part(1), Part(2), Part(3), Part(4));
        });
        println!("tuple, part {target}: caught {}", r.is_err());
        let r = panic::catch_unwind(|| drop(four()));
        println!("struct, part {target}: caught {}", r.is_err());
    }
    for target in [0, 1, 7, 20] {
        TARGET.set(target);
        let r = panic::catch_unwind(|| drop(twenty()));
        println!("twenty, part {target}: caught {}", r.is_err());
        let r = panic::catch_unwind(|| held(twenty(), Part(21)));
        println!("held twenty, part {target}: caught {}", r.is_err());
    }
}
