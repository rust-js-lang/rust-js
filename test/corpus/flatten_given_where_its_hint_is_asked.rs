// A flatten given to a function that asks its hint, `size_hint()` of its
// `I`, or given on to one, or through what gives it back: std's flatten counts its items' constant size,
// an array's, and an `Option`'s at most one, so it's held, std's struct,
// which knows them, where JS's own `flatMap` knew none and gave `(0,
// None)` (#1323). So are the adapters over it, a `map`, a `filter` and a
// `rev()`, and generic code's of its type parameter's items; the other
// adapters given there are as they were.
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn hints<I: Iterator<Item = u32>>(mut it: I) -> Vec<(usize, Option<usize>)> {
    let mut seen = vec![it.size_hint()];
    while it.next().is_some() {
        seen.push(it.size_hint());
    }
    seen
}

// Given on to `hint`, which asks it.
fn relayed<I: Iterator>(it: I) -> (usize, Option<usize>) {
    hint(it)
}

fn by_param<A: IntoIterator<Item = u32>, F: FnMut(u32) -> A>(f: F) -> (usize, Option<usize>) {
    (1..3).flat_map(f).size_hint()
}

fn main() {
    let v = vec![1u32, 2, 3, 4];
    println!("{:?}", hint(v.iter().copied().flat_map(|n| [n; 3])));
    println!("{:?}", hint(v.iter().copied().flat_map(Some)));
    println!("{:?}", hint(v.iter().map(|n| [*n; 2]).flatten()));
    println!("{:?}", relayed(v.iter().copied().flat_map(|n| [n; 3])));
    println!(
        "{:?}",
        hint(std::convert::identity(v.iter().copied().flat_map(|n| [n; 3])))
    );
    println!("{:?}", hint(v.iter().copied().flat_map(|n| [n; 3]).map(|x| x + 1)));
    println!(
        "{:?}",
        hint(v.iter().copied().flat_map(|n| [n; 3]).filter(|x| x % 2 == 0))
    );
    println!("{:?}", hint(v.iter().copied().flat_map(|n| [n; 3]).rev()));
    println!("{:?}", hints((1..3u32).flat_map(|n| [n; 3])));
    println!("{:?}", hint(v.iter().copied().filter(|n| n % 2 == 0)));
    println!("{:?}", hint(v.iter().copied().chain([5, 6])));
    println!("{:?}", hint(v.iter().copied().skip_while(|n| *n < 3)));
    println!("{:?}", hint(v.iter().copied().step_by(3)));
    println!("{:?}", hint(v.iter().copied().zip(0..2)));
    println!(
        "{:?}",
        hint(v.iter().copied().scan(0, |s, x| {
            *s += x;
            Some(*s)
        }))
    );
    println!("{:?}", by_param(|n| [n; 3]));
    println!("{:?}", by_param(Some));
}
