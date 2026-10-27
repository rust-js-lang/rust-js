// A zip held, std's `Zip` itself where std's iterators are compiled, which
// reads two `TrustedRandomAccess` sides by index (ADR 0975): a `map`'s,
// an `enumerate`'s, a `skip`'s, a `take`'s, a `fuse`'s and another zip's,
// stepped by `next()` and by `nth`, a side's closure running as std's zip
// runs it.
fn noisy(tag: &str, x: i32) -> i32 {
    println!("{tag}{x}");
    x
}

fn main() {
    let v = [1, 2, 3, 4];
    let w = [10, 20];

    let mut z = v.iter().map(|&x| noisy("a", x)).zip(w.iter());
    println!("{:?}", z.next());
    println!("{:?} {:?}", z.next(), z.next());

    let mut nth = v.iter().map(|&x| noisy("n", x)).zip(w.iter().enumerate());
    println!("{:?}", nth.nth(1));

    let mut skipped = v.iter().skip(1).map(|&x| noisy("s", x)).zip(w.iter().take(3).fuse());
    while let Some(pair) = skipped.next() {
        println!("{pair:?}");
    }

    let mut nested = v.iter().zip(w.iter()).zip(5..9);
    println!("{:?} {:?}", nested.next(), nested.next());
}
