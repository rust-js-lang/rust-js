// An adapter generic code makes of its own type parameter's iterator, given
// on to code that asks its hint (#1240): each `map` and `take` made knowing
// std's hint, of an array's items, a range and a `Vec`'s `into_iter()`
// alike, as the function is one for every instance its callers give, a
// range given with its dictionary, `$given`, as one too. And
// another generic function's `take` of the same type, made so too, still
// drops what's left of its source where a drop panics.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn doubled<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    hint(it.map(|x| x * 2))
}

fn first_two<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    hint(it.take(2).map(|x| x + 1))
}

fn rest<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    hint(it.skip(1).filter(|x| x % 2 == 0))
}

fn rest_cloned<I: Iterator<Item = u32> + Clone>(it: I) -> (usize, Option<usize>) {
    hint(it.clone().skip(1))
}

fn two_hinted<I: Iterator<Item = D>>(it: I) -> (usize, Option<usize>) {
    hint(it.take(2))
}

fn last_of_four<I: Iterator<Item = D>>(it: I) -> Option<u32> {
    it.take(4).map(|d| d.0).last()
}

fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?} {:?} {:?}", doubled(v.iter().copied()), doubled(4..9u32), doubled(v.clone().into_iter()));
    println!("{:?} {:?} {:?}", first_two(v.iter().copied()), first_two(0..10u32), first_two([7u32].into_iter()));
    println!("{:?} {:?} {:?}", rest(v.iter().copied()), rest(0..1u32), rest(Vec::<u32>::new().into_iter()));
    println!("{:?} {:?}", rest_cloned(v.iter().copied()), rest_cloned(0..1u32));
    let r = catch_unwind(AssertUnwindSafe(|| two_hinted(ds().into_iter())));
    println!("two hinted {:?}", r.ok());
    let r = catch_unwind(AssertUnwindSafe(|| last_of_four(ds().into_iter())));
    println!("last of four {:?}", r.ok());
}
