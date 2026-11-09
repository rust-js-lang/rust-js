// `repeat(x)`'s own `count` and `last` panic, "iterator is infinite", where
// std's default of its `next` would step it forever, and so as an adapter
// that passes either on asks it, a `skip`'s, an `enumerate`'s, a `fuse`'s, a
// `peekable`'s and a chain's second side's, and in generic code; what it
// holds dropped as the panic unwinds, as `count(self)` drops it (#1138).
use std::iter::repeat;
use std::panic::catch_unwind;
#[derive(Clone)]
struct L(u32);
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    println!("{}", catch_unwind(|| repeat(L(1)).count()).is_err());
    println!("{}", catch_unwind(|| repeat(L(2)).last().map(|l| l.0)).is_err());
    println!("{}", catch_unwind(|| repeat(L(3)).skip(2).count()).is_err());
    println!("{}", catch_unwind(|| repeat(L(4)).enumerate().count()).is_err());
    println!("{}", catch_unwind(|| repeat(L(5)).fuse().last().map(|l| l.0)).is_err());
    println!("{}", catch_unwind(|| repeat(7u32).peekable().count()).is_err());
    println!("{}", catch_unwind(|| [L(6)].into_iter().chain(repeat(L(7))).count()).is_err());
    println!("{}", catch_unwind(|| repeat(8u32).skip(1).last()).is_err());
    fn generic<T: Clone>(x: T) -> usize { std::iter::repeat(x).count() }
    println!("{}", catch_unwind(|| generic(L(9))).is_err());
    println!("end");
}
