//@ compile-fail: `clone_from` of `std::cmp::Reverse<std::result::Result<Counted, u8>>`, whose std `clone_from` is its value's
// `clone_from` of a `Reverse` of a `Result` whose value's `clone_from` is
// the crate's own, written directly: std's is its `Result`'s, of two `Ok`s
// the value's, which rust-js doesn't write inside a `Reverse` yet (#927):
// refused, not `*dest = source.clone()`, which would never call it.
use std::cmp::Reverse;

#[derive(Debug)]
struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        println!("clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Counted) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

fn main() {
    let mut r: Reverse<Result<Counted, u8>> = Reverse(Ok(Counted(1)));
    r.clone_from(&Reverse(Ok(Counted(2))));
    println!("{r:?}");
}
