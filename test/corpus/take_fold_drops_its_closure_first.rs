// A `take`'s `fold` gives what drops with its closure on to its source's
// `try_fold`, as std's `Take::fold` moves `f` into it: where a `skip`
// inside panics in its `nth(n - 1)`, its frame drops `f`, then the
// accumulator (#2196).
use std::panic::catch_unwind;

struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 3 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        panic!("nth {n}");
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let caught = catch_unwind(|| {
        let f = Loud("f");
        let acc = (Loud("acc"), 0);
        Count(0).fuse().skip(1).take(2).fold(acc, move |(held, a), x| {
            let _ = &f;
            (held, a + x)
        })
    });
    println!("caught {}", caught.is_err());
    let caught = catch_unwind(|| {
        let f = Loud("f2");
        let acc = (Loud("acc2"), 0);
        Count(0).skip(1).fold(acc, move |(held, a), x| {
            let _ = &f;
            (held, a + x)
        })
    });
    println!("caught {}", caught.is_err());
}
