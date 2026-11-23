//@ library-differs
// A zip one of whose sides is endless, `(1..4).zip(10..)` or
// `(10..).zip(1..4)`, has the other side's upper bound, `Some(3)`, as a
// crate's own `FromIterator` reads it, as std's `Zip` gives it (#1979's
// `zip-hint-endless-side-unbounded`).

struct Hinted(usize, Option<usize>);

impl FromIterator<(u32, u32)> for Hinted {
    fn from_iter<I: IntoIterator<Item = (u32, u32)>>(items: I) -> Self {
        let it = items.into_iter();
        let (low, high) = it.size_hint();
        Hinted(low, high)
    }
}

fn main() {
    let Hinted(low, high) = (1..4).zip(10..).collect();
    println!("{low} {high:?}");
    let Hinted(low, high) = (10..).zip(1..4).collect();
    println!("{low} {high:?}");
    let Hinted(low, high) = (1..4).zip(10..20).collect();
    println!("{low} {high:?}");
}
