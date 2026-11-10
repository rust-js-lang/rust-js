// Generic code's `collect()` into a `B: FromIterator` is std's collect of
// the instance its caller gives, `B = Vec<u32>` the `Vec`'s: it asks the
// iterator's hint as it grows, as the same collect written into a `Vec`
// does (#1239). The dictionary's `from_iter` is that one lowering.
use std::collections::{HashSet, VecDeque};

struct Loud(u32, u32);

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.0);
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

fn collect<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let direct: Vec<u32> = Loud(0, 3).collect();
    println!("{direct:?}");
    let generic: Vec<u32> = collect(Loud(0, 3));
    println!("{generic:?}");
    let filtered: Vec<u32> = collect([1u32, 2, 3, 4, 5].into_iter().filter(|x| x % 2 == 1));
    println!("{filtered:?}");
    let deque: VecDeque<u32> = collect(Loud(5, 7));
    println!("{deque:?}");
    let text: String = collect("abc".chars().rev());
    println!("{text}");
    let set: HashSet<u32> = collect(Loud(0, 2));
    let mut set: Vec<u32> = set.into_iter().collect();
    set.sort();
    println!("{set:?}");
}
