//@ compile-fail: which it's also given as
// A type parameter generic code holds in a place of its own (#2255), an
// iterator JS holds as no object, whose values it's given as it is and as
// the items of another iterator, `S: Iterator<Item = I>`: the items come as
// their caller holds them, bare, where its dictionaries read a place, so
// the call is refused (#2534) until each item is made a place where it
// crosses (#2535).

#[derive(Debug, Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let was = *self as u8;
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(was)
    }
}

#[derive(Debug, Clone, Copy)]
struct Bag;

#[derive(Debug, Clone)]
struct Empty;

impl Iterator for Empty {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl IntoIterator for Bag {
    type Item = u8;
    type IntoIter = Empty;

    fn into_iter(self) -> Empty {
        Empty
    }
}

#[derive(Debug, Clone)]
struct Counter {
    n: u8,
}

impl Iterator for Counter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.n += 1;
        Some(self.n)
    }
}

fn item_or<S: Iterator<Item = I>, I: Iterator>(mut s: S, or: I) -> I {
    s.next().unwrap_or(or)
}

fn two<S: Iterator<Item = J>, J: IntoIterator>(mut s: S, first: J) -> (J::IntoIter, J::IntoIter) {
    (first.into_iter(), s.next().unwrap().into_iter())
}

fn main() {
    let mut d = item_or([Light::Green].into_iter(), Light::Red);
    let mut e = item_or([].into_iter(), Light::Red);
    println!("item_or {:?} {:?} {d:?} {e:?}", d.next(), e.next());
    let mut c = item_or(vec![Counter { n: 3 }].into_iter(), Counter { n: 9 });
    println!("item_or {:?} {}", c.next(), c.n);

    let (mut a, mut b) = two([Light::Red].into_iter(), Light::Green);
    println!("two {:?} {:?} {a:?} {b:?}", a.next(), b.next());
    let (mut c, mut d) = two([Bag].into_iter(), Bag);
    println!("two {:?} {:?} {:?}", c.next(), d.next(), d.size_hint());
    let (mut e, mut f) = two(vec![Counter { n: 1 }].into_iter(), Counter { n: 5 });
    println!("two {:?} {:?} {} {}", e.next(), f.next(), e.n, f.n);
}
