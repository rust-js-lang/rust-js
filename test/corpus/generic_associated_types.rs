// A generic associated type is a type only a caller knows, as an associated
// type is (ADR 0146): its own lifetimes are erased, as JS has none, and a
// bound on one is in its trait's dictionary, one for every `Iter<'a>`. One
// with a const parameter and no bound is the type it is where it's used.
// In generic code, an `Iter<'a>` may be an array or a lazy JS iterator, as
// a type parameter may, and a `&mut` to one is the JS iterator it steps.
trait Container {
    type Item<'a>: std::fmt::Debug
    where
        Self: 'a;
    type Iter<'a>: Iterator<Item = Self::Item<'a>>
    where
        Self: 'a;
    fn items<'a>(&'a self) -> Self::Iter<'a>;
    fn last<'a>(&'a self) -> Option<Self::Item<'a>> {
        self.items().last()
    }
}

struct Shelf {
    books: Vec<String>,
}

impl Container for Shelf {
    type Item<'a> = &'a String;
    type Iter<'a> = std::slice::Iter<'a, String>;
    fn items<'a>(&'a self) -> Self::Iter<'a> {
        self.books.iter()
    }
}

struct Counts(Vec<u32>);

impl Container for Counts {
    type Item<'a> = u32;
    type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, u32>>;
    fn items<'a>(&'a self) -> Self::Iter<'a> {
        self.0.iter().copied()
    }
}

struct Countdown(u32);

impl Container for Countdown {
    type Item<'a> = u32;
    type Iter<'a> = std::iter::Successors<u32, fn(&u32) -> Option<u32>>;
    fn items<'a>(&'a self) -> Self::Iter<'a> {
        std::iter::successors(Some(self.0), |n: &u32| n.checked_sub(1))
    }
}

fn count<C: Container>(c: &C) -> usize {
    c.items().count()
}

fn first<C: Container>(c: &C) -> Option<C::Item<'_>> {
    c.items().next()
}

fn advance<I: Iterator>(it: &mut I) {
    it.next();
}

fn second<C: Container>(c: &C) -> Option<C::Item<'_>> {
    let mut items = c.items();
    advance(&mut items);
    items.next()
}

fn show<C: Container>(c: &C) -> String {
    format!("{:?}", c.last())
}

trait Filled {
    type Array<const N: usize>;
    fn filled<const N: usize>(&self) -> Self::Array<N>;
}

struct Byte(u8);

impl Filled for Byte {
    type Array<const N: usize> = [u8; N];
    fn filled<const N: usize>(&self) -> [u8; N] {
        [self.0; N]
    }
}

// An associated type bound by `Iterator` an impl gives a range, or an
// iterator of the crate's, for: a JS iterator in generic code.
trait Source {
    type Iter: Iterator<Item = u32>;
    fn numbers(&self) -> Self::Iter;
}

struct Upto(u32);

impl Source for Upto {
    type Iter = std::ops::Range<u32>;
    fn numbers(&self) -> std::ops::Range<u32> {
        0..self.0
    }
}

struct Down(u32);

impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
}

struct From3;

impl Source for From3 {
    type Iter = Down;
    fn numbers(&self) -> Down {
        Down(3)
    }
}

fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    let mut n = 0;
    for x in it {
        n += x;
    }
    n
}

fn summed<S: Source>(s: &S) -> u32 {
    total(s.numbers())
}

fn main() {
    let shelf = Shelf { books: vec!["Dune".to_string(), "Emma".to_string()] };
    let counts = Counts(vec![3, 1, 4]);
    println!("{:?} {:?}", shelf.last(), counts.last());
    println!("{} {} {}", count(&shelf), count(&counts), count(&Countdown(3)));
    println!("{:?} {} {}", Countdown(2).last(), show(&shelf), show(&Countdown(1)));
    println!("{:?} {:?} {:?}", first(&shelf), first(&counts), first(&Countdown(5)));
    println!("{:?} {:?} {:?}", second(&shelf), second(&counts), second(&Countdown(5)));
    println!("{:?}", shelf.items().map(|b| b.len()).collect::<Vec<_>>());
    println!("{:?} {:?}", Byte(7).filled::<3>(), Byte(1).filled::<0>());
    println!("{} {} {:?}", summed(&Upto(4)), summed(&From3), Upto(2).numbers());
}
