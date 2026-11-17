// `is_sorted_by_key` of keys with a destructor over a `take` whose source
// holds what has one: std's `is_sorted_by` holds the last key past the
// iterator's end, dropping it as its `all` returns, then drops the
// iterator, and with it the source. A `take` at its limit, a `$closing`
// done, mustn't close the source first (#2027, ADR 0570's held past the
// end).

#[derive(Debug)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("dropk {}", self.0);
    }
}

fn main() {
    // A `repeat`'s value, dropped with the `Take`.
    println!("{}", std::iter::repeat(L(7)).take(3).is_sorted_by_key(|l| K(l.0)));
    // A `Vec`'s rest, past the `take`'s limit.
    println!("{}", vec![L(1), L(2), L(3)].into_iter().take(2).is_sorted_by_key(|l| K(l.0)));
    // A `take` held in a variable first, moved into the consumer.
    let taken = vec![L(4), L(5), L(6)].into_iter().take(2);
    println!("{}", taken.is_sorted_by_key(|l| K(l.0)));
    // Out of order: the key that breaks it, then the last, then the rest.
    println!("{}", vec![L(9), L(8), L(3)].into_iter().take(3).is_sorted_by_key(|l| K(l.0)));
}
