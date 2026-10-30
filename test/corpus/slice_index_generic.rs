//@ library-differs
// `Index` and `IndexMut` of an index of a type parameter's, `I:
// SliceIndex<[T]>`, smallvec's: its dictionary's `index` and `index_mut`,
// the runtime's for `usize` and each kind of range (ADR 0418).
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

struct Small {
    items: Vec<u32>,
}

impl<I: SliceIndex<[u32]>> Index<I> for Small {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        &self.items[..][index]
    }
}

impl<I: SliceIndex<[u32]>> IndexMut<I> for Small {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        &mut self.items[..][index]
    }
}

fn main() {
    let mut s = Small { items: vec![1, 2, 3, 4] };
    s[1] = 20;
    s[2..].sort_by(|a, b| b.cmp(a));
    s[..=1][0] += 5;
    println!("{} {:?} {:?} {:?} {}", s[1], &s[1..3], &s[..], s.items, s[..=1].len());
}
