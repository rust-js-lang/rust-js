// A clone of a lazy `map` or `filter` is where its source is, with a clone
// of its closure, which changes what it took on its own: made again of a
// clone of each, by its clone, a `T: Clone`'s and a derived `Clone`'s too
// (ADR 0525).
fn both<I: Iterator<Item = i32> + Clone>(it: I) -> (Vec<i32>, Vec<i32>) {
    (it.clone().collect(), it.collect())
}

fn scaled<I: Iterator<Item = i32> + Clone>(it: I, k: i32) -> impl Iterator<Item = i32> + Clone {
    it.map(move |x| x * k)
}

#[derive(Clone)]
struct Holder<I> {
    it: I,
}

fn main() {
    let mut calls = 0;
    let base = 10;
    let it = (1..4).map(move |x| {
        calls += 1;
        x * base + calls
    });
    let mut copy = it.clone();
    println!("{:?}", copy.next());
    println!("{:?}", it.collect::<Vec<_>>());
    println!("{:?}", copy.collect::<Vec<_>>());

    let evens = vec![1, 2, 3, 4, 5, 6].into_iter().filter(|x| x % 2 == 0).map(|x| x * 3);
    let mut a = evens.clone();
    a.next();
    println!("{:?} {:?}", a.collect::<Vec<_>>(), evens.collect::<Vec<_>>());

    let mut seen = 0;
    let odd = [1, 2, 3, 4, 5].iter().copied().filter(move |x| {
        seen += 1;
        x % 2 == 1 && seen < 5
    });
    println!("{:?}", both(odd));

    // A `copied()` of a map over a slice, cloned apart: its clone is what it
    // copies' (ADR 0480).
    let doubled = [1, 2, 3, 4].iter().map(|x| x).copied().filter(|x| x % 2 == 0);
    let mut half = doubled.clone();
    half.next();
    println!("{:?} {:?}", half.collect::<Vec<_>>(), doubled.collect::<Vec<_>>());

    let tripled = scaled(1..4, 3);
    let mut rest = tripled.clone();
    rest.next();
    println!("{:?} {:?}", rest.collect::<Vec<_>>(), tripled.collect::<Vec<_>>());

    let holder = Holder { it: (0..3).map(|x| x + 100) };
    let mut again = holder.clone();
    again.it.next();
    println!("{:?} {:?}", again.it.collect::<Vec<_>>(), holder.it.collect::<Vec<_>>());
}
