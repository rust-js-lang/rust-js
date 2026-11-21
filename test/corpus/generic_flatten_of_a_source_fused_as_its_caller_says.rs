// Generic code's flatten of a source only its caller knows, `I`, a `map` of
// it, or one given on to another generic function, drops the source and its
// function where std's `Fuse` does by what the caller gives: of a `Vec`'s
// items, fused, as the flatten drops; of the crate's own iterator, at its
// first `None`. Each caller says whether its `I` is fused, `IFused`, as
// std's own `Fuse` of it asks (#2242, #2243); so of one held through a
// `&mut I`, made in place, told by its caller's `IFused` (#2242).
struct D(u32);
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
}
fn flattened<I: Iterator<Item = D>>(it: I, g: D) {
    let mut f = it.flat_map(move |d| {
        let _g = &g;
        vec![D(d.0 * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("ended, held");
    drop(f);
}
fn mapped<I: Iterator<Item = D>>(it: I, g: D) {
    let mut f = it.map(|d| d).flat_map(move |d| {
        let _g = &g;
        vec![D(d.0 * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("mapped ended, held");
    drop(f);
}
fn passed<I: Iterator<Item = D>>(it: I, g: D) {
    flattened(it, g);
}
fn held_lent<I: Iterator<Item = D>>(it: &mut I, g: D) {
    let mut f = it.flat_map(move |d| {
        let _g = &g;
        vec![D(d.0 * 10)]
    });
    while let Some(x) = f.next() {
        println!("got {}", x.0);
    }
    println!("lent ended, held");
    drop(f);
}

fn main() {
    flattened(vec![D(1), D(2)].into_iter(), D(91));
    flattened(Every { at: 1, end: 3 }, D(92));
    passed(vec![D(3), D(4)].into_iter(), D(93));
    passed(Every { at: 3, end: 5 }, D(94));
    mapped(vec![D(5), D(6)].into_iter(), D(95));
    mapped(Every { at: 5, end: 7 }, D(96));
    flattened(vec![D(7), D(8), D(9)].into_iter().take(1), D(97));
    held_lent(&mut vec![D(10), D(11)].into_iter(), D(98));
    held_lent(&mut Every { at: 12, end: 14 }, D(99));
}
