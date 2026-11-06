// Generic code's `extend` of a collection its caller gives, through its
// dictionary, where nothing the crate's own `size_hint` runs reaches a
// collection that asks it as it fills (#849, the user's (b)): a `Vec` by
// std's iterators or by the crate's that keeps std's default hint, and a
// hash set by the crate's own hint, which it asks once, first. Where a
// caller gives a `Vec` and an iterator whose hint is the crate's own, it's
// refused there, or relayed to the caller that knows them.
use std::collections::HashSet;

struct Quiet(u32);

impl Iterator for Quiet {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

struct Loud(u32);

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.0);
        (0, None)
    }
}

fn fill<C: Extend<u32>, I: IntoIterator<Item = u32>>(c: &mut C, items: I) {
    c.extend(items);
}

fn outer<C: Extend<u32> + Default, I: IntoIterator<Item = u32>>(items: I) -> C {
    let mut c = C::default();
    fill(&mut c, items);
    c
}

struct Buf {
    v: Vec<u32>,
}

impl Buf {
    fn fill<I: Iterator<Item = u32>>(&mut self, items: I) {
        self.v.extend(items);
    }
}

fn letters<C: Extend<char>>(c: &mut C, n: u32) {
    c.extend(Loud(n).map(|x| char::from(b'a' + x as u8)));
}

fn main() {
    let mut v: Vec<u32> = Vec::new();
    fill(&mut v, [1, 2, 3]);
    fill(&mut v, (0..4).map(|x| x * 2));
    fill(&mut v, Quiet(3));
    println!("vec {:?}", v);
    let mut s: HashSet<u32> = HashSet::new();
    fill(&mut s, Loud(4));
    println!("set {}", s.len());
    let w: Vec<u32> = outer(Quiet(2));
    let t: HashSet<u32> = outer(Loud(3));
    println!("outer {:?} {}", w, t.len());
    let mut text = String::from("z");
    letters(&mut text, 3);
    println!("text {text}");
    let mut b = Buf { v: vec![9] };
    b.fill(0..3);
    b.fill(Quiet(2));
    println!("buf {:?}", b.v);
}
