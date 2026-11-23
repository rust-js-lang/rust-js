//@ library-differs
// A crate's own `FromIterator` or `Sum` that asks the hint of what it's
// given sees std's, as native Rust's, called by name or by `collect()` and
// `sum()`, which call it from std's body: a `Vec`'s `take(n)`, `skip(n)` or
// `filter(..)` made in place, or into a `Result` of one, std's
// `GenericShunt` of it, whose JS iterator, `values().take(n)`, knows no
// hint (#2376, found by a probe of #1979's survivor `take-zero-asks-source`).
use std::iter::Sum;

struct Hinted(usize, Option<usize>);

impl FromIterator<u32> for Hinted {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.into_iter().size_hint();
        Hinted(low, high)
    }
}

struct Room(usize);

impl Sum<u32> for Room {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        Room(items.size_hint().0)
    }
}

// The crate's `from_iter` called by its name, where no `collect()` makes
// what it's given.
fn called_by_name() {
    let Hinted(low, high) = Hinted::from_iter(vec![1, 2, 3].into_iter().take(2));
    println!("from_iter called {low} {high:?}");
}

fn main() {
    for n in [0, 1, 5] {
        let Hinted(low, high) = vec![1, 2, 3].into_iter().take(n).collect();
        println!("take({n}) {low} {high:?}");
    }
    let Hinted(low, high) = vec![1, 2, 3].into_iter().skip(1).collect();
    println!("skip {low} {high:?}");
    called_by_name();
    let Hinted(low, high) = vec![1, 2, 3].into_iter().filter(|&x| x > 1).collect();
    println!("filter {low} {high:?}");
    let checked: Result<Hinted, String> = vec![Ok(1), Ok(2), Ok(3)].into_iter().skip(1).collect();
    if let Ok(Hinted(low, high)) = checked {
        println!("collected checked {low} {high:?}");
    }
    let Room(room) = vec![4, 5, 6].into_iter().skip(2).sum();
    println!("sum's room {room}");
    let v: Vec<u32> = vec![1, 2, 3].into_iter().take(2).collect();
    println!("{v:?}");
}
