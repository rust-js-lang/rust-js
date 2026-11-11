//! A consumer that asks none of what the library's functions it compiles
//! itself ask (#1325).
#[derive(Debug)]
struct Loud {
    n: u32,
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.n);
    }
}

pub fn main() {
    println!("{}", dep::rest_after_one(Loud { n: 1 }, "abc"));
    println!("{}", dep::pretty(Loud { n: 2 }));
}
