//@ library-differs
// A field of the crate's own iterator that its `Iterator` impl holds in a
// place of its own, `a: I` of `Both<I, J>` given a fieldless enum's or a
// unit struct's iterator, which JS holds as no object (#2255): its place,
// `{ value: "A" }`, is the field wherever `Both<Tri, Tri>` is, so what
// reads, copies, clones, compares, hashes, defaults, matches or drops the
// struct by its shape reads what's in the place and makes a place of its
// own: a copy or a clone shares none, `match k.a` tests what's in it.

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Both<I, J> {
    a: I,
    b: J,
}

impl<I: Iterator<Item = u8>, J: Iterator<Item = u8>> Iterator for Both<I, J> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(self.a.next()? * 10 + self.b.next()?)
    }
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum Step<I> {
    On(I),
    Off,
}

impl<I: Iterator> Iterator for Step<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match self {
            Step::On(it) => it.next(),
            Step::Off => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Two<I> {
    A(I),
    B(Tri),
}

impl<I: Iterator<Item = u8>> Iterator for Two<I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        match self {
            Two::A(it) => it.next(),
            Two::B(t) => t.next(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
struct Wrap<I>(I, u8);

impl<I: Iterator<Item = u8>> Iterator for Wrap<I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(self.0.next()? + self.1)
    }
}

#[derive(Debug, Clone)]
struct Duo<I>(I, Noisy);

impl<I: Iterator<Item = u8>> Iterator for Duo<I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0.next()
    }
}

/// Gives 4s, a unit struct.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Blank;

impl Iterator for Blank {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(4)
    }
}

fn blanks_of(Both { a, b }: Both<Blank, Blank>) -> (Option<Blank>, bool) {
    (Some(a), a == b)
}

/// Its own `PartialEq`, which a derived one of what holds it calls.
#[derive(Debug, Clone, Copy)]
enum Odd {
    X,
    Y,
}

impl PartialEq for Odd {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Odd::X, Odd::X) | (Odd::Y, Odd::Y))
    }
}

impl Iterator for Odd {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        *self = Odd::Y;
        Some(1)
    }
}

#[derive(Debug, PartialEq)]
struct Pin<I>(I);

impl<I: Iterator<Item = u8>> Iterator for Pin<I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0.next()
    }
}

const START: Both<Tri, Tri> = Both { a: Tri::A, b: Tri::B };

fn first_of(Wrap(t, _): Wrap<Tri>) -> Tri {
    t
}

fn a_of(Both { a, .. }: Both<Tri, Tri>) -> Tri {
    a
}

/// An iterator whose clone says so.
#[derive(Debug)]
struct Noisy(u8);

impl Clone for Noisy {
    fn clone(&self) -> Self {
        println!("clone noisy");
        Noisy(self.0)
    }
}

impl Iterator for Noisy {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        Some(self.0)
    }
}

#[derive(Debug, Clone)]
struct Pair<I> {
    left: I,
    noisy: Noisy,
}

impl<I: Iterator<Item = u8>> Iterator for Pair<I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(self.left.next()? + self.noisy.next()?)
    }
}

#[derive(Debug, Clone)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Iterator<Item = u8>, R: Iterator<Item = u8>> Iterator for Either<L, R> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        match self {
            Either::Left(l) => l.next(),
            Either::Right(r) => r.next(),
        }
    }
}

struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud");
    }
}

impl Iterator for Loud {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(9)
    }
}

fn main() {
    {
        let mut l = Both { a: Loud, b: Tri::A };
        println!("{:?}", l.next());
        let Both { a, b } = l;
        println!("{b:?}");
        drop(a);
        let _m = Both { a: Loud, b: Loud };
    }
    let mut p = Both { a: Tri::A, b: Tri::C };
    let mut q = p.clone();
    let r = p;
    println!("{:?} {:?} {p:?} {q:?} {r:?}", p.next(), q.next());
    println!("{} {} {:?}", p == q, p < r, Ord::cmp(&p, &r));
    let mut set = HashSet::new();
    set.insert(p);
    set.insert(q);
    set.insert(r);
    println!("{}", set.len());
    let d: Both<Tri, Tri> = Both::default();
    let e = Both { a: Tri::B, ..d };
    let Both { a, b } = e;
    println!("{d:?} {e:?} {a:?} {b:?}");
    let mut m = Both { a: Tri::C, b: Tri::C };
    let n = std::mem::replace(&mut m.a, Tri::B);
    std::mem::swap(&mut m.a, &mut m.b);
    println!("{n:?} {m:?} {:?}", m.next());
    let mut o = OneOf::It(Tri::B);
    let mut o2 = o.clone();
    println!("{:?} {:?} {o:?} {o2:?} {}", o.next(), o2.next(), o == o2);
    let v = vec![Both { a: Tri::A, b: Tri::B }; 2];
    let mut w = v.clone();
    w[0].next();
    println!("{v:?} {w:?}");
    let mut t = (Both { a: Tri::C, b: Tri::A }, 1);
    let t2 = t;
    t.0.next();
    println!("{t:?} {t2:?}");
    let s: Vec<u8> = Both { a: Tri::A, b: Tri::A }.take(3).collect();
    println!("{s:?}");
    let k = Both { a: Tri::B, b: Tri::C };
    let named = match k.a {
        Tri::A => "a",
        Tri::B => "b",
        Tri::C => "c",
    };
    let deep = matches!(k, Both { a: Tri::B, b: Tri::C });
    println!("{named} {deep} {}", k.a as u8);
    let mut on = Step::On(Tri::A);
    let kept = on;
    println!("{:?} {on:?} {kept:?} {}", on.next(), on > kept);
    let mut s = Pair {
        left: Tri::A,
        noisy: Noisy(0),
    };
    let mut s2 = s.clone();
    println!("{:?} {:?} {s:?} {s2:?}", s.next(), s2.next());
    let mut x: Either<Tri, Noisy> = Either::Left(Tri::B);
    let y = x.clone();
    println!("{:?} {x:?} {y:?}", x.next());
    let mut two: Two<Tri> = Two::B(Tri::C);
    let other = two;
    let mut one = Two::A(Tri::A);
    let first = one;
    println!("{:?} {:?} {two:?} {other:?} {one:?} {first:?}", two.next(), one.next());
    let mut w = Wrap(Tri::B, 1);
    let v = w;
    let d: Wrap<Tri> = Wrap::default();
    println!("{:?} {w:?} {v:?} {d:?} {} {}", w.next(), w == v, w > v);
    let Wrap(inner, _) = w;
    println!("{inner:?} {:?} {:?}", first_of(v), a_of(Both { a: Tri::C, b: Tri::A }));
    let mut duo = Duo(Tri::A, Noisy(5));
    let twin = duo.clone();
    println!("{:?} {duo:?} {twin:?}", duo.next());
    let (blank, same) = blanks_of(Both { a: Blank, b: Blank });
    println!("{blank:?} {} {same}", blank.is_some());
    let mut pin = Pin(Odd::X);
    println!("{} {:?} {}", pin == Pin(Odd::X), pin.next(), pin == Pin(Odd::Y));
    let mut start = START;
    println!("{:?} {start:?} {START:?}", start.next());
}
