// `C::from_iter(items)` of the crate's own iterator, which is what
// `items.collect()` calls: collected as `collect` collects it, its `next`
// stepping what it holds, in generic code, in a trait's default copied into
// an impl, into a `Vec`, a `String` and a `BTreeSet`, and given as an
// `IntoIterator`; and dropped where `collect` drops it, at its end, an
// iterator with a destructor given whole, generic code's by its caller's
// drop, and not one lent (ADR 0160, #838).
use std::collections::BTreeSet;

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

fn reversed<I: Iterator>(w: Wrapper<I>) -> Vec<I::Item> {
    let mut v = Vec::from_iter(w);
    v.reverse();
    v
}

trait Gather: Iterator + Sized {
    fn gathered(self) -> Vec<Self::Item> {
        Vec::from_iter(self)
    }
}

impl<I: Iterator> Gather for Wrapper<I> {}

fn sorted_set<J: IntoIterator<Item = u8>>(items: J) -> BTreeSet<u8> {
    BTreeSet::from_iter(items)
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counter(u32, &'static str);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

fn gathered<T: Iterator>(t: T) -> Vec<T::Item> {
    let v = Vec::from_iter(t);
    println!("gathered");
    v
}

fn given<J: IntoIterator<Item = u32>>(j: J) -> Vec<u32> {
    let v = Vec::from_iter(j);
    println!("given");
    v
}

fn main() {
    println!("{:?}", reversed(Wrapper([4, 5, 6].into_iter())));
    println!("{:?}", Vec::from_iter(Wrapper([1, 2].into_iter())));
    println!("{:?}", Wrapper("ab".chars()).gathered());
    println!("{:?}", String::from_iter(Wrapper(['x', 'y', 'z'].into_iter())));
    println!("{:?}", sorted_set(Wrapper([3, 1, 3, 2].into_iter())));
    println!("{:?}", sorted_set(vec![9, 7]));
    let louds = Vec::from_iter(Wrapper((1..=3).map(Loud)));
    println!("{}", louds.iter().map(|l| l.0).sum::<u32>());
    drop(louds);
    println!("{:?}", gathered(Counter(2, "generic")));
    println!("{:?}", given(Counter(2, "into")));
    println!("{:?}", Vec::from_iter(Counter(2, "whole")));
    let mut held = Counter(3, "held");
    println!("{:?}", Vec::from_iter(held.by_ref().take(1)));
    println!("{:?}", Vec::from_iter(&mut held));
    println!("end");
}
