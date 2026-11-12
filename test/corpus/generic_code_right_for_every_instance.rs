// Generic code whose one JS body decides by its type parameter alone (ADR
// 1115's Alone sites, #1170), each called with instances that differ, so
// the answer must be right for every one: a clone of a `copied()` of what
// it's given, a std iterator's, the crate's own and an adapter; a `Vec`
// collected, then extended, whose capacity the program reads, of items and
// iterators of each kind; a collection it doesn't know, extended; and a
// value moved out of a type parameter's place through a `&mut`, swapped.
use std::collections::{BTreeSet, VecDeque};

struct Own<'a> {
    v: &'a [u32],
    at: usize,
    tag: u8,
}

impl Clone for Own<'_> {
    fn clone(&self) -> Self {
        println!("clone own {}", self.tag);
        Own {
            v: self.v,
            at: self.at,
            tag: self.tag + 1,
        }
    }
}

impl<'a> Iterator for Own<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        let x = self.v.get(self.at);
        self.at += 1;
        x
    }
}

struct Odd {
    n: u32,
}

impl Iterator for Odd {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            Some(self.n)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, None)
    }
}

fn twice<'a, I: Iterator<Item = &'a u32> + Clone>(it: I) -> (u32, u32) {
    let copied = it.copied();
    let again = copied.clone();
    (copied.sum(), again.map(|x| x * 2).sum())
}

fn gather<T, I: Iterator<Item = T>>(it: I) -> Vec<T> {
    let v: Vec<T> = it.collect();
    println!("len {} cap {}", v.len(), v.capacity());
    v
}

fn widen<T, I: Iterator<Item = T>>(mut v: Vec<T>, it: I) -> Vec<T> {
    v.extend(it);
    println!("extended len {} cap {}", v.len(), v.capacity());
    v
}

fn fill<C: Extend<u32>, I: Iterator<Item = u32>>(c: &mut C, it: I) {
    c.extend(it);
}

fn swap_in<T>(a: &mut T, b: &mut T) {
    std::mem::swap(a, b);
}

fn main() {
    let v = [1, 2, 3];
    println!("{:?}", twice(v.iter()));
    println!("{:?}", twice(Own { v: &v, at: 1, tag: 0 }));
    println!("{:?}", twice(v.iter().skip(1)));

    let a = gather([1u32, 2, 3].into_iter());
    let b = gather(Odd { n: 5 });
    let c = gather((0..7u64).map(|x| (x, x)).filter(|p| p.0 % 2 == 0));
    let d = gather(vec![String::from("x"); 3].into_iter());
    let e = gather((0..9).filter(|x| x % 3 == 0).map(|x| [x; 4]));
    println!("{} {} {} {} {}", a.len(), b.len(), c.len(), d.len(), e.len());
    let f = widen(a, Odd { n: 9 });
    let g = widen(b, 0..20);
    let h = widen(c, (0..30u64).filter(|x| x % 2 == 1).map(|x| (x, x)));
    println!("{} {} {}", f.len(), g.len(), h.len());

    let mut w: Vec<u32> = Vec::new();
    fill(&mut w, 0..5);
    println!("{} {}", w.len(), w.capacity());
    fill(&mut w, (0..9).filter(|x| x % 2 == 0));
    println!("{} {}", w.len(), w.capacity());
    let mut q: VecDeque<u32> = VecDeque::new();
    fill(&mut q, [7, 8].into_iter());
    println!("{} {}", q.len(), q.capacity() >= 2);
    let mut s = BTreeSet::new();
    fill(&mut s, 3..6);
    println!("{s:?}");

    let (mut x, mut y) = (vec![1], vec![2, 3]);
    swap_in(&mut x, &mut y);
    let (mut m, mut n) = (4u8, 5u8);
    swap_in(&mut m, &mut n);
    let mut pairs = [(1, 'a'), (2, 'b')];
    let (first, rest) = pairs.split_at_mut(1);
    swap_in(&mut first[0], &mut rest[0]);
    println!("{x:?} {y:?} {m} {n} {pairs:?}");
}
