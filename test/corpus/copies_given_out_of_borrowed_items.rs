//@ library-differs
// What a function gives copies of, of items it borrows, each a value of
// its own: a copy changed leaves the item it was made of as it was. In
// the metamorphic net's `library` rewrite these functions are a library's,
// whose JS copies only what its own crate changes, so the crate that uses
// them compiles each that copies what it changes itself (ADR 0914,
// amended): an iterator that gives copies, as it's stepped, of what it
// borrows, by its `Item`, not only a value that holds one (#2014).
#[derive(Clone, Copy, Debug)]
pub struct P(pub u32, pub u32);

pub fn copies(ps: &[P]) -> std::iter::Copied<std::slice::Iter<'_, P>> {
    ps.iter().copied()
}

pub fn clones(ps: &[P]) -> std::iter::Cloned<std::slice::Iter<'_, P>> {
    ps.iter().cloned()
}

pub fn derefs(ps: &[P]) -> impl Iterator<Item = P> + '_ {
    ps.iter().map(|p| *p)
}

pub fn first(ps: &[P]) -> Option<P> {
    ps.first().copied()
}

pub fn evens(ps: &[P]) -> impl Iterator<Item = P> + '_ {
    ps.iter().filter(|p| p.0 % 2 == 0).copied()
}

fn main() {
    let ps = [P(1, 10), P(2, 20), P(3, 30), P(4, 40)];
    let mut a = copies(&ps).next().unwrap();
    a.1 = 0;
    let mut b = clones(&ps).nth(1).unwrap();
    b.0 = 0;
    let mut c = derefs(&ps).last().unwrap();
    c.1 += 1;
    let mut d = first(&ps).unwrap();
    d.0 += 5;
    let mut e: Vec<P> = evens(&ps).collect();
    e[0].1 = 7;
    println!("{a:?} {b:?} {c:?} {d:?} {e:?}");
    println!("{ps:?}");
}
