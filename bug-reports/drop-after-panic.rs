// A panic while dropping one part of an aggregate must still drop the rest.
// Native rustc does. rust-js does not: the later destructor never runs.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Boom;

impl Drop for Boom {
    fn drop(&mut self) {
        println!("boom");
        panic!("boom");
    }
}

struct Pair {
    first: Boom,
    second: Loud,
}

fn main() {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _pair = Pair {
            first: Boom,
            second: Loud(1),
        };
    }));
    println!("caught {}", caught.is_err());
}
