// Generic code whose callers aren't all seen, a `pub` function's, clones
// what its parameters may be, any type: its `cycle()` of an `I` is no
// `map` or `filter` it isn't given, and the `map` and `filter` it gives
// behind an `impl Iterator + Clone`, each closure its caller's instance,
// are those its caller clones, the filter cloned apart (ADR 0525; #1119).
// Nor is a `map` of items that are `Clone` and have a destructor, which its
// `I: Iterator + Clone` allows, but which is no type parameter (#1318).
#[derive(Clone, Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn firsts<I: Iterator + Clone>(iter: I, n: usize) -> Vec<I::Item> {
    iter.cycle().take(n).collect()
}

pub fn scaled<I: Iterator<Item = i32> + Clone>(it: I, k: i32) -> impl Iterator<Item = i32> + Clone {
    it.map(move |x| x * k)
}

pub fn evens_of<I: Iterator<Item = i32> + Clone>(it: I) -> impl Iterator<Item = i32> + Clone {
    it.filter(|x| x % 2 == 0)
}

fn main() {
    println!("{:?}", firsts(0..3, 7));
    println!("{:?}", firsts(vec!["a", "b"].into_iter(), 3));

    // Neither cloned, each closure counting by a `&mut`, which isn't `Clone`.
    let mut calls = 0;
    let mapped: Vec<i32> = (1..4)
        .map(|x| {
            calls += 1;
            x * 10 + calls
        })
        .filter(|x| x % 2 == 0)
        .collect();
    println!("{mapped:?} {calls}");

    let tripled = scaled(1..4, 3);
    let mut rest = tripled.clone();
    rest.next();
    println!("{:?} {:?}", rest.collect::<Vec<_>>(), tripled.collect::<Vec<_>>());

    let even = evens_of(1..7);
    let mut later = even.clone();
    later.next();
    println!("{:?} {:?}", later.collect::<Vec<_>>(), even.collect::<Vec<_>>());
    let louds: Vec<Loud> = vec![Loud(1), Loud(2)].into_iter().map(|l| Loud(l.0 + 10)).collect();
    println!("{louds:?}");
}
