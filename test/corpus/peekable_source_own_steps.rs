// A compiled `Peekable`'s or `Fuse`'s source that is a JS iterator, a
// chain's or a `Rev`'s, is stepped by its own `last` and `nth`, std's for
// its kind, so what each passes is dropped in std's order (#276, fuzz
// seeds 51951, 50899 and 51010).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    // `Chain::last`: each side's `last`, the first side's dropped after.
    let last = louds(10, 2).into_iter().chain(louds(20, 2)).peekable().last();
    println!("{:?}", last.map(|loud| loud.0));
    // `Rev::nth`: a `Vec`'s `nth_back`, what it passes dropped as one slice.
    let nth = louds(30, 2).into_iter().rev().peekable().nth(id(3));
    println!("{:?}", nth.map(|loud| loud.0));
    let nth = louds(40, 2).into_iter().rev().fuse().peekable().nth(id(2));
    println!("{:?}", nth.map(|loud| loud.0));
}
