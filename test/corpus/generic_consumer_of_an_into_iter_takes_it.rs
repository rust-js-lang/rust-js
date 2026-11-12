// Generic code's consumer of a `T::IntoIter`, its `into_iter()`'s
// iterator, takes it: `for_each`, `sum`, `count`, `last`, `max` and `fold`
// reach the crate's own `fold`, which takes it and drops it, so the body
// drops nothing after them; one whose steps are std's is dropped after
// (#1590). As generic code given a type parameter's iterator does (#522).
// And a `fuse()` or a `map()` of one, summed there or by generic code it's
// given to: its dictionary has the value's own `fold`, `$foldOf`, which a
// compiled `Fuse` folds by, and which took it (#1595).
struct Up {
    n: u32,
}

impl Iterator for Up {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("fold {}", self.n);
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

struct Steps {
    n: u32,
}

impl Iterator for Steps {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl Drop for Steps {
    fn drop(&mut self) {
        println!("drop Steps {}", self.n);
    }
}

fn each<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    let mut sum = 0;
    iter.into_iter().for_each(|x| sum += x);
    sum
}

fn summed<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    iter.into_iter().sum()
}

fn counted<T: IntoIterator<Item = u32>>(iter: T) -> usize {
    iter.into_iter().count()
}

fn last<T: IntoIterator<Item = u32>>(iter: T) -> Option<u32> {
    iter.into_iter().last()
}

fn most<T: IntoIterator<Item = u32>>(iter: T) -> Option<u32> {
    iter.into_iter().max()
}

fn folded<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    iter.into_iter().fold(0, |a, x| a * 10 + x)
}

fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.sum()
}

fn fused<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    iter.into_iter().fuse().sum()
}

fn passed_fused<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    total(iter.into_iter().fuse())
}

fn passed_mapped<T: IntoIterator<Item = u32>>(iter: T) -> u32 {
    total(iter.into_iter().map(|x| x + 1))
}

fn main() {
    println!("each {}", each(Up { n: 0 }));
    println!("summed {}", summed(Up { n: 1 }));
    println!("counted {}", counted(Up { n: 0 }));
    println!("last {:?}", last(Up { n: 0 }));
    println!("most {:?}", most(Up { n: 0 }));
    println!("folded {}", folded(Up { n: 0 }));
    println!("each {}", each(Steps { n: 0 }));
    println!("summed {}", summed(Steps { n: 1 }));
    println!("counted {}", counted(Steps { n: 0 }));
    println!("last {:?}", last(Steps { n: 0 }));
    println!("most {:?}", most(Steps { n: 0 }));
    println!("folded {}", folded(Steps { n: 0 }));
    println!("each {}", each(vec![1, 2]));
    println!("fused {}", fused(Up { n: 1 }));
    println!("passed_fused {}", passed_fused(Up { n: 1 }));
    println!("passed_mapped {}", passed_mapped(Up { n: 1 }));
    println!("fused {}", fused(Steps { n: 1 }));
    println!("passed_fused {}", passed_fused(Steps { n: 1 }));
    println!("passed_mapped {}", passed_mapped(Steps { n: 1 }));
}
