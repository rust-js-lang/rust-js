// A std value's `Debug`, a `Vec`'s, an `Option`'s or a tuple's, of an item
// whose `Debug` prints a stdout line then writes to stderr, as `println!`
// formats: written piece by piece through std's builders' model, so what
// Rust's stdout had written by the stderr write, `list [N` then `in 1`, is
// known and written first (#1490). test/stream-order.test.ts holds the two
// streams' order against native.
use std::fmt;
struct Noisy(u32);
impl fmt::Debug for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "N")?;
        println!("in {}", self.0);
        eprintln!("err {}", self.0);
        write!(f, "{}", self.0)
    }
}
fn main() {
    println!("list {:?} end", vec![Noisy(1), Noisy(2)]);
    println!("opt {:?} end", Some(Noisy(3)));
    println!("pair {:?} end", (Noisy(4), 5u8));
    println!("{:#?}", vec![Noisy(6)]);
    println!("done");
}
