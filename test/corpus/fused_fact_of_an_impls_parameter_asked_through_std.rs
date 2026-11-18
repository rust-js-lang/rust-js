// An impl's method, `rfold` of a `DoubleEndedIterator`, that steps a `Fuse`
// of the impl's own type parameter from its back through std's adapters,
// `a.by_ref().rev()`, as itertools' `ZipLongest::rfold` does: std's `Fuse`
// steps by its source's `FusedIterator`-ness, which no call in the method
// names, only std's body it reaches (#2042). Of a `FusedIterator` source
// holding what drops, `Fuse` keeps it past its first `None` and asks it
// again; of one that isn't, it drops it there and asks nothing more.
// Its `rfold` of `&mut Fuse<T>` is std's default, stepping by `Fuse`'s own
// `next_back`, the method std specializes.
use std::iter::{Fuse, FusedIterator};

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Src {
    left: u32,
    held: Loud,
}

impl Iterator for Src {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.next_back()
    }
}

impl DoubleEndedIterator for Src {
    fn next_back(&mut self) -> Option<u32> {
        if self.left == 0 {
            println!("{} asked past its end", self.held.0);
            return None;
        }
        self.left -= 1;
        Some(self.left)
    }
}

impl ExactSizeIterator for Src {
    fn len(&self) -> usize {
        self.left as usize
    }
}

impl FusedIterator for Src {}

/// A source that isn't a `FusedIterator`: `Fuse` drops it at its first
/// `None`.
struct Plain(Src);

impl Iterator for Plain {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
}

impl DoubleEndedIterator for Plain {
    fn next_back(&mut self) -> Option<u32> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for Plain {
    fn len(&self) -> usize {
        self.0.len()
    }
}

struct Pair<T> {
    a: Fuse<T>,
}

fn pair<T: Iterator>(a: T) -> Pair<T> {
    Pair { a: a.fuse() }
}

impl<T: DoubleEndedIterator + ExactSizeIterator> Iterator for Pair<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<T::Item> {
        self.a.next()
    }
}

impl<T: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Pair<T> {
    fn next_back(&mut self) -> Option<T::Item> {
        self.a.next_back()
    }

    fn rfold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let Self { mut a } = self;
        let n = a.len() / 2;
        let init = a.by_ref().rev().take(n).fold(init, &mut f);
        let init = a.by_ref().rev().fold(init, &mut f);
        println!("drained");
        let again = a.by_ref().rev().next();
        println!("again {}", again.is_some());
        init
    }
}

fn pushed(mut acc: Vec<u32>, x: u32) -> Vec<u32> {
    acc.push(x);
    acc
}

fn main() {
    let fused = pair(Src {
        left: 4,
        held: Loud("fused"),
    })
    .rfold(Vec::new(), pushed);
    println!("{fused:?}");
    let plain = pair(Plain(Src {
        left: 3,
        held: Loud("plain"),
    }))
    .rfold(Vec::new(), pushed);
    println!("{plain:?}");
}
