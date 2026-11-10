// `eq()` and `ne()` of an iterator an `impl Iterator` hides, compared as
// the type it hides is: two `TrustedLen` ones of unequal lengths compared
// by their lengths first, nothing stepped, as std's `SpecIterEq` sees
// through the opaque type (#1229); one that isn't, item by item. In
// generic code too, where the hidden type is made of its type parameter,
// which its caller decides.
#[derive(PartialEq)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn items(v: Vec<D>) -> impl Iterator<Item = D> {
    v.into_iter()
}

fn hidden<I: Iterator<Item = D>>(it: I) -> impl Iterator<Item = D> {
    it
}

fn compare<I: IntoIterator<Item = D>>(a: I, b: Vec<D>) -> bool {
    hidden(a.into_iter()).eq(items(b))
}

fn odd(n: &u32) -> bool {
    println!("odd {n}");
    n % 2 == 1
}

fn filtered(to: u32) -> impl Iterator<Item = u32> {
    (1..to).filter(odd)
}

fn main() {
    println!("{}", items(vec![D(1), D(2), D(3)]).eq(items(vec![D(1), D(2)])));
    println!("{}", items(vec![D(4), D(5)]).ne(vec![D(4), D(6)]));
    println!("{}", hidden(vec![D(7)].into_iter()).eq(hidden(vec![D(7), D(8)].into_iter())));
    println!("{}", compare(vec![D(9), D(10)], vec![D(9)]));
    println!("{}", compare(vec![D(11)], vec![D(11)]));
    println!("{}", filtered(6).eq(filtered(4)));
    println!("{}", filtered(4).eq([1u32, 3]));
    println!("{}", (1..4u32).eq(hidden_range(4)));
}

fn hidden_range(to: u32) -> impl Iterator<Item = u32> {
    1..to
}
