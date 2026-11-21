//@ library-differs
// An iterator of the crate's that JS holds as no object, a fieldless
// enum's string or a unit struct's `undefined`, whose own `&mut self`
// steps take a place: generic code that holds an iterator as the value
// itself is given it in a place of its own, `{ value: x }`, and its
// dictionaries read the place, a `clone()` making a new one (#2255). By
// value to std's adapters and the crate's generic functions, as a `&mut`
// to generic code that clones it and writes it back (#2254), lent through
// a trait's `&mut T`, through a `dyn`, and stepped by its own `nth`.
// And held in a field of the crate's own iterator, `a: I` of `Both<I, J>`,
// whose `Iterator` impl holds it so: in that place wherever `Both<Tri, _>`
// is, built by its own code or generic code, read, written, lent as a
// `&mut` and cloned, and a variant's, `OneOf::It(I)`.

use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
enum Tri {
    #[default]
    A,
    B,
    C,
}

impl Iterator for Tri {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let (n, to) = match self {
            Tri::A => (1, Tri::B),
            Tri::B => (2, Tri::C),
            Tri::C => (3, Tri::A),
        };
        *self = to;
        Some(n)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint of {self:?}");
        (3, None)
    }
}

#[derive(Debug, Copy)]
struct Ticks;

impl Clone for Ticks {
    fn clone(&self) -> Self {
        println!("clone ticks");
        Ticks
    }
}

impl Iterator for Ticks {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(7)
    }
}

fn look<I: Iterator<Item = u8> + Clone + Debug + PartialEq + Default + Ord>(mut it: I) -> I {
    it.next();
    let again = it.clone();
    println!("{it:?} {again:?} {:?}", it.size_hint());
    it.next();
    println!("{} {} {:?}", it == again, it > again, I::default());
    let mut all = vec![it.clone(), again, I::default()];
    all.sort();
    println!("{all:?}");
    it
}

struct Back<'a, I> {
    iter: &'a mut I,
}

impl<I: Iterator + Clone> Back<'_, I> {
    fn peek_back(&mut self) -> Option<I::Item> {
        let old = self.iter.clone();
        let item = self.iter.next();
        *self.iter = old;
        item
    }

    fn step(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

fn back<I: Iterator + Clone>(iter: &mut I) -> Back<'_, I> {
    Back { iter }
}

trait Lend<T> {
    fn lend(&mut self) -> &mut T;
}

impl<T> Lend<T> for T {
    fn lend(&mut self) -> &mut T {
        self
    }
}

fn back_through<I: Iterator + Clone>(iter: &mut impl Lend<I>) -> Back<'_, I> {
    Back { iter: iter.lend() }
}

#[derive(Debug, Clone)]
struct Both<I, J> {
    a: I,
    b: J,
}

impl<I: Iterator<Item = u8>, J: Iterator<Item = u8>> Iterator for Both<I, J> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let a = self.a.next()?;
        Some(a * 10 + self.b.next()?)
    }
}

fn both<I: Iterator<Item = u8>, J: Iterator<Item = u8>>(a: I, b: J) -> Both<I, J> {
    Both { a, b }
}

fn step_one<I: Iterator>(it: &mut I) -> Option<I::Item> {
    it.next()
}

#[derive(Debug)]
enum OneOf<I> {
    It(I),
    Done,
}

impl<I: Iterator> Iterator for OneOf<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match self {
            OneOf::It(it) => it.next(),
            OneOf::Done => None,
        }
    }
}

fn main() {
    let t = look(Tri::A);
    println!("{t:?}");

    let v: Vec<u8> = Tri::B.take(4).collect();
    let w: Vec<u8> = Ticks.take(2).collect();
    println!("{v:?} {w:?}");
    let mut p = Tri::A.peekable();
    let first = p.peek().copied();
    let next = p.next();
    println!("{first:?} {next:?} {:?}", p.peek());
    let c: Vec<u8> = Tri::C.take(2).cycle().take(5).collect();
    let z: Vec<(u8, u8)> = Tri::A.zip(Tri::C).take(3).collect();
    let ch: Vec<u8> = Tri::A.take(1).chain(Tri::B.take(2)).collect();
    println!("{c:?} {z:?} {ch:?}");
    let f: Vec<u8> = Some(Tri::B).into_iter().flat_map(|t| t.take(2)).collect();
    let e: Vec<(usize, u8)> = Tri::C.enumerate().skip(1).step_by(2).take(2).collect();
    println!("{f:?} {e:?} {:?}", Tri::A.position(|n| n == 3));

    let mut l = Tri::A;
    println!("{:?} {:?} {l:?}", l.next(), l.nth(1));
    let s: u32 = l.by_ref().take(2).map(u32::from).sum();
    println!("{s} {l:?}");
    let mut b: Box<dyn Iterator<Item = u8>> = Box::new(Tri::B);
    println!("{:?} {:?}", b.next(), b.next());

    let mut l = Tri::A;
    let stepped = back(&mut l).step();
    println!("{stepped:?} {l:?}");
    let peeked = back(&mut l).peek_back();
    println!("{peeked:?} {l:?}");
    let stepped = back_through(&mut l).step();
    let peeked = back_through(&mut l).peek_back();
    println!("{stepped:?} {peeked:?} {l:?}");
    let mut u = Ticks;
    let peeked = back(&mut u).peek_back();
    println!("{peeked:?} {u:?}");

    let mut pair = Both { a: Tri::B, b: Ticks };
    println!("{:?} {:?} {pair:?}", pair.next(), pair.a);
    pair.a = Tri::A;
    let copied = pair.a;
    let looked = &pair.a;
    println!("{copied:?} {looked:?}");
    println!("{:?} {pair:?}", step_one(&mut pair.a));
    let lent = &mut pair.a;
    let stepped = lent.next();
    println!("{stepped:?} {:?} {pair:?}", pair.next());
    let again = pair.clone();
    let made = both(Tri::C, Tri::A);
    let taken: Vec<u8> = made.clone().take(2).collect();
    println!("{again:?} {made:?} {taken:?} {:?}", look(made.a));
    let mut one = OneOf::It(Tri::C);
    if let OneOf::It(it) = &mut one {
        println!("{:?} {it:?}", it.next());
    }
    println!("{:?} {one:?} {:?}", one.next(), OneOf::<Tri>::Done.next());
}
