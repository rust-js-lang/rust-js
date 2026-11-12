// Generic code's `count()` of a `T::IntoIter`, its `into_iter()`'s
// iterator, is the iterator's own `count` where it has one, which takes it
// and drops it, as a type parameter's is: it was folded by `reduce`, the
// own `count` never called and the iterator never dropped (#1602). Of one
// whose `count` is std's, folded and dropped after. Stepped first, held in
// an `Option` or a `Vec`, or counted after a `by_ref()`'s.
struct Counts {
    n: u32,
}

impl Iterator for Counts {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
    fn count(self) -> usize {
        println!("own count {}", self.n);
        (3 - self.n) as usize
    }
}

impl Drop for Counts {
    fn drop(&mut self) {
        println!("drop Counts {}", self.n);
    }
}

/// Its own `fold` too, which a `count` of std's would reach.
struct Both {
    n: u32,
}

impl Iterator for Both {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
    fn count(self) -> usize {
        println!("own count {}", self.n);
        (3 - self.n) as usize
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("own fold {}", self.n);
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }
}

impl Drop for Both {
    fn drop(&mut self) {
        println!("drop Both {}", self.n);
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

fn counted<T: IntoIterator>(iter: T) -> usize {
    iter.into_iter().count()
}

fn counted_after_one<T: IntoIterator>(iter: T) -> usize {
    let mut it = iter.into_iter();
    it.next();
    it.count()
}

fn counted_from_an_option<T: IntoIterator>(iter: T) -> usize {
    let held = Some(iter.into_iter());
    held.unwrap().count()
}

fn counted_from_a_vec<T: IntoIterator>(iter: T) -> usize {
    let mut held = vec![iter.into_iter()];
    held.pop().map_or(0, |it| it.count())
}

fn counted_past_a_by_ref<T: IntoIterator>(iter: T) -> usize {
    let mut it = iter.into_iter();
    let first = it.by_ref().take(1).count();
    first + it.count()
}

fn counted_param<I: Iterator>(it: I) -> usize {
    it.count()
}

fn main() {
    println!("counted {}", counted(Counts { n: 0 }));
    println!("counted {}", counted(Both { n: 0 }));
    println!("counted {}", counted(Steps { n: 0 }));
    println!("counted {}", counted(vec![1, 2]));
    println!("after one {}", counted_after_one(Counts { n: 0 }));
    println!("after one {}", counted_after_one(Both { n: 0 }));
    println!("after one {}", counted_after_one(Steps { n: 0 }));
    println!("from an option {}", counted_from_an_option(Counts { n: 1 }));
    println!("from an option {}", counted_from_an_option(Steps { n: 1 }));
    println!("from a vec {}", counted_from_a_vec(Both { n: 1 }));
    println!("from a vec {}", counted_from_a_vec(Steps { n: 1 }));
    println!("past a by_ref {}", counted_past_a_by_ref(Counts { n: 0 }));
    println!("past a by_ref {}", counted_past_a_by_ref(Both { n: 0 }));
    println!("param {}", counted_param(Both { n: 0 }));
}
