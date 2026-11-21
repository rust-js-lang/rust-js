// A boxed source's `flat_map` that holds a function with a destructor
// drops it where std's `FlatMap` does: folded, `count`, `fold` or `rfold`
// of one the crate also steps from its back, before what's left of the
// source, as `Map::fold` and `Map::rfold` move it into the source's fold,
// where it dropped after; and dropped with an inner iterator part-stepped,
// after the source and before the inner's items, as `FlatMap`'s drop glue
// drops its `Map` before its `frontiter` and its `backiter`, where it
// dropped after them (#2241).
struct D(u32);
impl D {
    fn v(&self) -> u32 {
        self.0
    }
}
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Every {
    at: u32,
    end: u32,
}
impl Drop for Every {
    fn drop(&mut self) {
        println!("drop src");
    }
}
impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
    fn count(mut self) -> usize {
        println!("own count");
        let mut n = 0;
        while self.next().is_some() {
            n += 1;
        }
        n
    }
    fn last(mut self) -> Option<D> {
        println!("own last");
        let mut last = None;
        while let Some(x) = self.next() {
            last = Some(x);
        }
        last
    }
}
impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }
}

fn pair(g: D) -> impl FnMut(D) -> Vec<D> {
    move |x| {
        let _g = &g;
        vec![D(x.v() * 10), D(x.v() * 10 + 1)]
    }
}
fn main() {
    let n = Box::new(Every { at: 1, end: 4 }).flat_map(pair(D(92))).count();
    println!("ends count {n}");
    let s = Box::new(Every { at: 1, end: 4 })
        .flat_map(pair(D(93)))
        .fold(0, |a, d| a + d.v());
    println!("ends fold {s}");
    let s = Box::new(Every { at: 1, end: 4 })
        .flat_map(pair(D(94)))
        .rfold(0, |a, d| a + d.v());
    println!("ends rfold {s}");
    let b = Box::new(Every { at: 1, end: 4 })
        .flat_map(pair(D(83)))
        .next_back()
        .map(|d| d.v());
    println!("ends back {b:?}");
    let b = Box::new(Every { at: 1, end: 4 })
        .flat_map(pair(D(84)))
        .next()
        .map(|d| d.v());
    println!("ends next {b:?}");
    let g = D(85);
    let b = Box::new(Every { at: 1, end: 4 })
        .flat_map(move |x| {
            let _g = &g;
            vec![D(x.v() * 10), D(x.v() * 10 + 1)]
        })
        .next()
        .map(|d| d.v());
    println!("fwd next {b:?}");
}
