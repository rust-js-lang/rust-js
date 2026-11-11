// A struct's field of a `&mut T`, or of a `T` given a `&mut`, holds one
// value wherever the struct is: generic code's box, a struct made where `T`
// is known holding its object's in-place cell, so a generic function that
// gives one back, a guard, a slot and regex-syntax's parser holder each
// write the caller's place (#1405).
use std::borrow::BorrowMut;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

struct Shape {
    at: Point,
    name: String,
}

// A slot made by a generic function, written by generic methods and
// through its field where `T` is known.
struct Slot<'a, T> {
    at: &'a mut T,
}

impl<T: Copy> Slot<'_, T> {
    fn set(&mut self, v: T) {
        *self.at = v;
    }

    fn get(&self) -> T {
        *self.at
    }
}

// Reached through a `&mut` to the slot, where `T` is known.
fn poke(slot: &mut Slot<'_, Point>, by: i32) -> i32 {
    slot.at.x += by;
    let seen = &slot.at;
    seen.x + slot.at.y
}

fn hold<T>(at: &mut T) -> Slot<'_, T> {
    Slot { at }
}

fn release<T>(slot: Slot<'_, T>) -> &mut T {
    slot.at
}

// A struct made where `T` is known, given to a generic function that
// gives back what it holds.
struct Holder<'a, T> {
    r: &'a mut T,
}

fn get<'a, T>(h: Holder<'a, T>) -> &'a mut T {
    h.r
}

fn bump_through(shape: &mut Shape) {
    get(Holder { r: &mut shape.at }).x += 100;
}

// A guard that writes back on drop, of any `T`.
struct Guard<'a, T: Debug> {
    target: &'a mut T,
    log: Vec<String>,
}

impl<T: Debug> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        println!("dropped over {:?} after {:?}", self.target, self.log);
    }
}

fn guard<T: Debug>(target: &mut T) -> Guard<'_, T> {
    Guard { target, log: Vec::new() }
}

// regex-syntax's shape: a holder of `P: BorrowMut<Parser>`, given a
// `&mut Parser`.
#[derive(Debug, Default)]
struct Parser {
    depth: u32,
    seen: Vec<char>,
}

struct ParserI<'s, P> {
    parser: P,
    pattern: &'s str,
}

impl<'s, P: BorrowMut<Parser>> ParserI<'s, P> {
    fn new(parser: P, pattern: &'s str) -> ParserI<'s, P> {
        ParserI { parser, pattern }
    }

    fn parse(&mut self) -> usize {
        let parser = self.parser.borrow_mut();
        for c in self.pattern.chars() {
            if c == '(' {
                parser.depth += 1;
            }
            parser.seen.push(c);
        }
        parser.seen.len()
    }
}

// An enum of one, matched where `T` is known.
enum Edit<'a, T> {
    Replace(&'a mut T, T),
    Keep,
}

fn apply<T>(edit: Edit<'_, T>) -> bool {
    match edit {
        Edit::Replace(at, v) => {
            *at = v;
            true
        }
        Edit::Keep => false,
    }
}

// A holder of a `&mut` to an iterator and one to a function, read by
// impls bound by `Iterator` and `FnMut`.
struct Counted<'a, I> {
    inner: &'a mut I,
    count: usize,
}

impl<I: Iterator> Iterator for Counted<'_, I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let item = self.inner.next()?;
        self.count += 1;
        Some(item)
    }
}

struct Calling<'a, F> {
    f: &'a mut F,
}

impl<F: FnMut(i32) -> i32> Calling<'_, F> {
    fn call(&mut self, x: i32) -> i32 {
        (self.f)(x)
    }
}

fn main() {
    // A slot of a field, written by its generic methods and where `T` is
    // known, read whole and through a shared borrow.
    let mut shape = Shape { at: Point { x: 1, y: 2 }, name: "box".to_string() };
    let mut slot = hold(&mut shape.at);
    slot.set(Point { x: 5, y: 6 });
    slot.at.x += 10;
    *slot.at = Point { x: slot.at.y, y: slot.at.x };
    let seen: &&mut Point = &slot.at;
    println!("{:?} {:?}", seen, slot.get());
    println!("{}", poke(&mut slot, 7));
    let back = release(slot);
    back.y -= 1;
    println!("{:?} {}", shape.at, shape.name);
    bump_through(&mut shape);
    bump_through(&mut shape);
    println!("{:?}", shape.at);

    // A slot made where `T` is known, re-pointed whole, then taken apart.
    let mut a = Point { x: 0, y: 0 };
    let mut b = Point { x: 9, y: 9 };
    let mut slot = Slot { at: &mut a };
    slot.set(Point { x: 1, y: 1 });
    slot.at = &mut b;
    slot.set(Point { x: 2, y: 2 });
    let Slot { at } = slot;
    at.x = 3;
    println!("{a:?} {b:?}");

    // A `&mut` to the field itself, the slot re-pointed through it.
    let mut c = Point { x: 0, y: 0 };
    let mut d = Point { x: 0, y: 0 };
    let mut slot = Slot { at: &mut c };
    let r = &mut slot.at;
    *r = &mut d;
    r.x = 1;
    slot.set(Point { x: slot.get().x, y: 5 });
    println!("{c:?} {d:?}");

    // A number's slot, a cell either way.
    let mut n = 3u32;
    let mut count = hold(&mut n);
    count.set(7);
    *count.at += 1;
    let got = count.get();
    println!("{got} {n}");

    // A guard over a vector, written through while held.
    let mut items = vec![1, 2];
    {
        let mut g = guard(&mut items);
        g.target.push(3);
        g.log.push("pushed".to_string());
        g.target[0] = 10;
    }
    println!("{items:?}");

    // regex-syntax's holder of a `&mut Parser`, and of a `Parser` it owns.
    let mut parser = Parser::default();
    let n = ParserI::new(&mut parser, "(a(b))").parse();
    let m = ParserI::new(&mut parser, "c(").parse();
    let owned = ParserI::new(Parser::default(), "((").parse();
    println!("{n} {m} {owned} {parser:?}");

    // An enum's variant made where `T` is known.
    let mut p = Point { x: 1, y: 1 };
    let applied = apply(Edit::Replace(&mut p, Point { x: 4, y: 4 }));
    let kept = apply::<Point>(Edit::Keep);
    println!("{applied} {kept} {p:?}");
    let mut q = Point { x: 0, y: 0 };
    if let Edit::Replace(at, v) = Edit::Replace(&mut q, Point { x: 8, y: 8 }) {
        at.y = v.x + 1;
    }
    println!("{q:?}");

    // An iterator and a function behind a `&mut` in a struct.
    let mut words = vec!["a", "b", "c"].into_iter();
    let mut counted = Counted { inner: &mut words, count: 0 };
    let first: Vec<&str> = counted.by_ref().take(2).collect();
    println!("{first:?} {} {:?}", counted.count, words.next());
    let mut total = 0;
    let mut add = |x: i32| {
        total += x;
        total
    };
    let mut calling = Calling { f: &mut add };
    let one = calling.call(2);
    let two = calling.call(3);
    println!("{one} {two} {total}");
}
