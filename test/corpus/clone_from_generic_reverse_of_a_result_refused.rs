//@ compile-fail: a `Clone` dictionary of `std::cmp::Reverse<std::result::Result<Counted, u8>>` given to generic code, whose std `clone_from` is its value's
// A `Clone` dictionary of a `Reverse` of a `Result` whose value's
// `clone_from` is the crate's own, given to generic code: std's
// `clone_from` of it is the `Result`'s, of two `Ok`s the value's, which a
// `Reverse`'s value written in place doesn't reach yet (#927, #2028):
// refused, never `*dest = source.clone()` (#928).
use std::cmp::Reverse;
struct Counted(u32);
impl Clone for Counted {
    fn clone(&self) -> Counted {
        Counted(self.0)
    }
    fn clone_from(&mut self, source: &Counted) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}
fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}
fn main() {
    let mut w: Reverse<Result<Counted, u8>> = Reverse(Ok(Counted(1)));
    assign(&mut w, &Reverse(Ok(Counted(2))));
    println!("{}", matches!(w.0, Ok(Counted(2))));
}
