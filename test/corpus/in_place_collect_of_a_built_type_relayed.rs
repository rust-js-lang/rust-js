// A collect into a `Vec` of items built of a function's type parameters,
// `Held<<H::Item as IntoIterator>::IntoIter>`, asked of a generic caller,
// which asks it of its own of a type built of its own, `Held<IntoIter<T>>`;
// and of a trait's default copied into a blanket impl, in the impl's terms,
// as itertools' `Itertools::multi_cartesian_product` collects. Where a panic
// stops it, in place it drops what it collected first, else what's left
// (#672, #737), each item's iterator too, an associated type of the
// default's own where clause (#798).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I> {
    iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

fn held<H>(iters: H) -> Vec<Held<<H::Item as IntoIterator>::IntoIter>>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    iters.map(|i| Held { iter: i.into_iter() }).collect()
}

/// Of a type of its own the layout of which is its caller's.
fn relayed<T, J: Iterator<Item = Option<T>>>(options: J) -> usize {
    held(options).len()
}

/// Of a source std never collects in place, whose layout it never reads,
/// and asks none of its caller.
fn ranged<T: Clone>(t: T) -> usize {
    held((1..=2).map(move |_| Some(t.clone()))).len()
}

/// Of a type with a destructor of its own, holding an associated type of
/// its parameter no caller gives a drop of, items of no destructor.
fn holding<J: Iterator>(items: J) -> Vec<Held<J::Item>> {
    items.map(|iter| Held { iter }).collect()
}

/// Of a tuple built of its own, `(usize, I::Item)`.
fn enumerated<I: Iterator>(items: I) -> Vec<(usize, I::Item)> {
    items.enumerate().collect()
}

trait Holding: Iterator {
    fn held_all(self) -> Vec<Held<<Self::Item as IntoIterator>::IntoIter>>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.map(|i| Held { iter: i.into_iter() }).collect()
    }
}

impl<T: Iterator> Holding for T {}

/// Of the blanket impl given its own type parameter's iterator, which
/// isn't one of its type parameters, `<I as IntoIterator>::IntoIter`, as
/// itertools' `free::sorted` calls `Itertools::sorted`.
fn held_of<I: IntoIterator>(items: I) -> usize
where
    I::Item: IntoIterator,
{
    items.into_iter().held_all().len()
}

fn wide(w: Option<D>) -> Option<D> {
    if w.as_ref().is_some_and(|d| d.0 == 3) {
        panic!("wide");
    }
    w
}

fn narrow(v: Vec<D>) -> Vec<D> {
    if v[0].0 == 3 {
        panic!("narrow");
    }
    v
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("relayed", || relayed(vec![Some(D(1)), None].into_iter()));
    run("relayed, a panic", || {
        let wides: Vec<Option<D>> = (1..=4).map(|n| Some(D(n))).collect();
        relayed(wides.into_iter().map(wide))
    });
    run("relayed, not in place", || relayed((1..=2).map(|n| Some(D(n)))));
    run("relayed, of nothing to drop", || relayed((1..=2).map(Some)));
    run("ranged", || ranged(7u8));
    run("holding", || holding([1u8, 2].into_iter()).len());
    run("enumerated", || {
        let pairs = enumerated(vec![D(8), D(9)].into_iter());
        pairs.iter().map(|(at, d)| at + d.0 as usize).sum()
    });
    run("held of", || {
        let wides: Vec<Option<D>> = (1..=2).map(|n| Some(D(n))).collect();
        held_of(wides)
    });
    run("a default, a panic", || {
        let wides: Vec<Option<D>> = (1..=4).map(|n| Some(D(n))).collect();
        wides.into_iter().map(wide).held_all().len()
    });
    run("a default, narrow, a panic", || {
        let narrows: Vec<Vec<D>> = (1..=4).map(|n| vec![D(n)]).collect();
        narrows.into_iter().map(narrow).held_all().len()
    });
    run("a default, not in place", || {
        let held = (1..=2).map(|n| vec![D(n)]).held_all();
        held.iter().map(|h| h.iter.len()).sum()
    });
}
