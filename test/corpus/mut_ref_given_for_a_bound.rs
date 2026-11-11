//@ library-differs
// A `&mut` given for a type parameter is generic code's box of what it's to
// (ADR 0454), so every dictionary of a `&mut` reads through it: std's
// `Borrow`, `BorrowMut`, `AsRef` and `AsMut` of one, each beside a shared
// reference's and an owned value's; std's impls that pass a trait on from a
// `&mut T` to its `T`, `Display`, `Debug` and `Hash`; and a `dyn` made of
// a `&mut`, which holds the box too, a crate's impl for `&mut T` and std's
// alike (#1414).
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct Parser {
    depth: u32,
}

impl Display for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.depth)
    }
}

fn bump<P: BorrowMut<Parser>>(mut p: P) -> u32 {
    let parser = p.borrow_mut();
    parser.depth += 1;
    parser.depth
}

fn peek<P: Borrow<Parser>>(p: P) -> u32 {
    p.borrow().depth
}

fn bump_n<P: BorrowMut<u32>>(mut p: P) -> u32 {
    *p.borrow_mut() += 5;
    *p.borrow()
}

fn peek_n<P: Borrow<u32>>(p: P) -> u32 {
    *p.borrow() + 1
}

fn fill<V: AsMut<[u8]>>(mut v: V) -> u8 {
    v.as_mut()[0] = 9;
    v.as_mut()[0]
}

fn first<V: AsRef<[u8]>>(v: V) -> u8 {
    v.as_ref()[0]
}

fn text_len<S: AsRef<str>>(s: S) -> usize {
    s.as_ref().len()
}

fn show<D: Display>(d: D) -> String {
    format!("[{d}]")
}

fn debugged<D: Debug>(d: D) -> String {
    format!("{d:?}")
}

fn hash_of<H: Hash>(h: H) -> u64 {
    let mut state = DefaultHasher::new();
    h.hash(&mut state);
    state.finish()
}

fn read_two<R: Read>(mut r: R) -> [u8; 2] {
    let mut two = [0; 2];
    r.read_exact(&mut two).unwrap();
    two
}

// A crate's impl for `&mut T`, through generic code and a `dyn`.
trait Total {
    fn total(&self) -> u32;
}

impl Total for Vec<u32> {
    fn total(&self) -> u32 {
        self.iter().sum()
    }
}

impl<T: Total> Total for &mut T {
    fn total(&self) -> u32 {
        (**self).total() + 1
    }
}

fn totalled<T: Total>(t: T) -> u32 {
    t.total()
}

fn main() {
    let mut parser = Parser::default();
    let a = bump(&mut parser);
    let b = bump(Parser::default());
    let c = peek(&mut parser);
    let d = peek(&parser);
    let e = peek(parser.clone());
    println!("{a} {b} {c} {d} {e} {parser:?}");

    let mut n = 1u32;
    let mut three = 3u32;
    let f = bump_n(&mut n);
    let g = bump_n(&mut three);
    let h = peek_n(&mut n);
    let i = peek_n(&n);
    let j = peek_n(n);
    println!("{f} {g} {h} {i} {j} {n} {three}");

    let mut v = vec![1u8, 2];
    let mut arr = [5u8, 6];
    let k = fill(&mut v);
    let l = fill(vec![0u8, 1]);
    let m = fill(&mut arr);
    let o = first(&v);
    let p = first(&mut v);
    let q = first(v.clone());
    let r = first(&arr);
    println!("{k} {l} {m} {o} {p} {q} {r} {v:?} {arr:?}");

    let mut text = String::from("abc");
    println!("{} {} {} {}", text_len(&mut text), text_len(&text), text_len("hello"), text_len(text.clone()));

    // std's impls of a `&mut T` that pass the trait on to `T`.
    println!("{} {} {}", show(&mut parser), show(&parser), show(parser.clone()));
    println!("{} {} {}", debugged(&mut parser), debugged(&mut v), debugged(&mut 5));
    println!("{}", hash_of(&mut parser) == hash_of(&parser));
    let mut bytes = vec![7u8, 8, 9];
    println!("{:?} {:?}", read_two(Cursor::new(&mut bytes)), read_two(&mut Cursor::new(&bytes)));

    // A `dyn` of a `&mut`, a crate's impl's and std's.
    let mut totals = vec![1, 2];
    let through = totalled(&mut totals);
    let held = &mut totals;
    let as_dyn: &dyn Total = &held;
    println!("{through} {}", as_dyn.total());
    let at = &mut parser;
    let shown: &dyn Display = &at;
    let boxed: Box<dyn Debug + '_> = Box::new(&mut v);
    println!("{shown} {boxed:?}");
}
