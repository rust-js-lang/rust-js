// A `take`'s `count`, which takes it, drops it as a panic in what it steps
// unwinds, and with it what it holds: the crate's iterator whose `next`
// panics, under a `skip` or over one, under `catch_unwind` (#640).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own(u32);

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        if self.0 == 3 {
            panic!("next");
        }
        Some(D(self.0))
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

fn main() {
    let r = catch_unwind(|| Own(0).skip(1).take(3).count());
    println!("skip take count {}", r.is_err());
    let r = catch_unwind(|| Own(0).take(3).skip(1).count());
    println!("take skip count {}", r.is_err());
    let r = catch_unwind(|| Own(0).take(5).count());
    println!("take count {}", r.is_err());
    println!("ended {}", Own(10).take(2).count());
}
