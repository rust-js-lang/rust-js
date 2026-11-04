// What a consumer std routes through `fold` takes of a `skip`, as std
// takes it (#494): `last`, and `collect` into a `HashMap`, a `HashSet` or a
// `String`. Std's `Skip::fold` asks its source's `nth(n - 1)`, drops what
// that gives, then folds its source, the crate's own `fold` where it writes
// one, and a reversed `Vec`'s `nth_back` drops a slice front to back; a
// map's `extend` drops what it replaces as each comes, before what the
// take's source holds is dropped at its end; and a `String` collected of
// `String`s takes its first by `next`, a `skip`'s `nth(n)`. A `last`
// through a `skip` or a `chain` reaches the crate's own `last`, and drops
// `a`'s where `b` has one. And stepped by `next`, a `skip` of a
// `take_while` asks its `nth`, std's default by `try_fold`, so the inner
// `skip`'s `nth(n - 1)` (#294). A `LinkedList` collected of what has no
// destructor, a `skip`'s of what has, is by its fold too.
use std::collections::{HashMap, HashSet, LinkedList};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4)]
}

struct Folds(Vec<D>);

impl Iterator for Folds {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

struct Own(Vec<D>);

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

struct Lasts(Vec<D>);

impl Iterator for Lasts {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn last(mut self) -> Option<D> {
        println!("own last");
        self.0.pop()
    }
}

fn main() {
    let last = Folds(ds()).skip(2).last();
    println!("last {:?}", last.map(|d| d.0));
    let text: String = Folds(ds()).skip(1).map(|d| d.0.to_string()).collect();
    println!("folded {text}");
    let text: String = Own(ds()).take(3).skip(1).map(|d| d.0.to_string()).collect();
    println!("first by next {text}");
    let m: HashMap<u8, D> = Own(ds()).take(2).skip(0).map(|d| (1, d)).collect();
    println!("replaced {:?}", m.values().map(|d| d.0).collect::<Vec<_>>());
    drop(m);
    let m: HashMap<u8, D> = ds().into_iter().rev().take(3).skip(2).map(|d| (1, d)).collect();
    println!("reversed {:?}", m.values().map(|d| d.0).collect::<Vec<_>>());
    drop(m);
    let s: HashSet<u32> = ds().into_iter().rev().take(3).skip(2).map(|d| d.0).collect();
    println!("set {}", s.len());
    let list: LinkedList<u32> = ds().into_iter().rev().take(3).skip(2).map(|d| d.0).collect();
    println!("list {list:?}");
    let last = Lasts(ds()).skip(1).last();
    println!("own last {:?}", last.map(|d| d.0));
    let last = Lasts(ds()).chain(Lasts(vec![D(5), D(6)])).skip(1).last();
    println!("chained own last {:?}", last.map(|d| d.0));
    let mut it = ds().into_iter().rev().skip(2).take_while(|d| d.0 < 9).skip(2);
    println!("next {:?}", it.next().map(|d| d.0));
}
