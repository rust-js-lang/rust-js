// A boxed source's adapter that holds a function with a destructor,
// counted or asked its `last` where what it steps has its own, drops what
// it counts or passes and its function, as std's `Filter::count` sums a
// `map` of its own and `FlattenCompat::last` folds its `Map`, each moving
// the function into its source's fold, dropped there before what's left of
// the source: its own `count` and `last` forwarded were given neither the
// items' drop nor the function's, and taken, never dropped it; so of one
// the crate also steps from its back (#2224).
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

fn odd(g: D) -> impl FnMut(&D) -> bool {
    move |x| {
        let _g = &g;
        x.v() % 2 == 1
    }
}

fn pair(g: D) -> impl FnMut(D) -> Vec<D> {
    move |x| {
        let _g = &g;
        vec![D(x.v() * 10), D(x.v() * 10 + 1)]
    }
}

fn some_odd(g: D) -> impl FnMut(D) -> Option<D> {
    move |x| {
        let _g = &g;
        if x.v() % 2 == 1 { Some(x) } else { None }
    }
}

fn main() {
    let g1 = D(91);
    let n = Box::new(Every { at: 1, end: 6 })
        .filter(move |x| {
            let _g = &g1;
            x.v() % 2 == 1
        })
        .count();
    println!("filter count {n}");
    let g2 = D(92);
    let n = Box::new(Every { at: 1, end: 4 })
        .flat_map(move |x| {
            let _g = &g2;
            vec![D(x.v() * 10)]
        })
        .count();
    println!("flat_map count {n}");
    let g3 = D(93);
    let l = Box::new(Every { at: 1, end: 4 })
        .flat_map(move |x| {
            let _g = &g3;
            vec![D(x.v() * 10), D(x.v() * 10 + 1)]
        })
        .last()
        .map(|d| d.v());
    println!("flat_map last {l:?}");
    let g4 = D(94);
    let l = Box::new(Every { at: 1, end: 4 })
        .map(move |x| {
            let _g = &g4;
            x.v()
        })
        .last();
    println!("map last {l:?}");
    let n = Box::new(Every { at: 1, end: 6 }).filter(odd(D(71))).count();
    println!("filter count, stepped from its back {n}");
    let b = Box::new(Every { at: 1, end: 6 })
        .filter(odd(D(81)))
        .next_back()
        .map(|d| d.v());
    println!("filter back {b:?}");
    let l = Box::new(Every { at: 1, end: 4 })
        .flat_map(pair(D(73)))
        .last()
        .map(|d| d.v());
    println!("flat_map last, stepped from its back {l:?}");
    let b = Box::new(Every { at: 1, end: 1 })
        .flat_map(pair(D(83)))
        .next_back()
        .is_some();
    println!("flat_map back {b}");
    let n = Box::new(Every { at: 1, end: 6 }).filter_map(some_odd(D(74))).count();
    println!("filter_map count, stepped from its back {n}");
    let b = Box::new(Every { at: 1, end: 6 })
        .filter_map(some_odd(D(84)))
        .next_back()
        .map(|d| d.v());
    println!("filter_map back {b:?}");
}
