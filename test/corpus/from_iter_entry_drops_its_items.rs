// A crate's `FromIterator` impl whose `from_iter` collects what it's given
// into a `Vec`, called through its dictionary by generic code: its entry
// makes what the method asks of its own `I`, the items' drop and the buffer
// std collects in place, under the method's bounds (#1239), so each item
// collected, and what's left when a step panics, drops as Rust drops it.
use std::panic::{self, AssertUnwindSafe};

struct D(i32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Req {
    cs: Vec<D>,
}

impl FromIterator<D> for Req {
    fn from_iter<I: IntoIterator<Item = D>>(iter: I) -> Self {
        Req {
            cs: Vec::from_iter(iter),
        }
    }
}

fn gather<T: FromIterator<D>>(v: Vec<D>, stop: i32) -> T {
    v.into_iter()
        .map(|d| {
            if d.0 == stop {
                panic!("at {stop}")
            } else {
                D(d.0 * 10)
            }
        })
        .collect()
}

fn main() {
    let req: Req = gather(vec![D(1), D(2), D(3)], 0);
    println!("gathered {}", req.cs.len());
    drop(req);
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        let _: Req = gather(vec![D(1), D(2), D(3), D(4)], 3);
    }));
    println!("caught {}", caught.is_err());
}
