//@ library-differs
// A tuple of iterators JS holds as no object, a fieldless enum's string and
// a unit struct's `undefined`, taken or given back by value where generic
// code holds each in a place of its own, `{ value: x }`: it crosses part by
// part, `[{ value: a }, { value: b }]`, as itertools' `multizip((a, b))`
// gives one to its `From<(A, B)>` impl (#2255). By a direct call and
// through an impl's dictionary, a tuple's own impl of a trait; a part
// generic code holds as it is, a number, as it is. And a blanket impl's
// dictionary given generic code, whose entries take the iterator in the
// place their own generic code holds it, as the generic code that holds
// the dictionary does, so it's given as it is.

#[derive(Debug, Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(*self as u8)
    }
}

/// Gives sevens, a unit struct.
#[derive(Debug)]
struct Sevens;

impl Iterator for Sevens {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(7)
    }
}

fn firsts<A: Iterator, B: Iterator>((mut a, mut b): (A, B)) -> (Option<(A::Item, B::Item)>, (A, B)) {
    let first = a.next().zip(b.next());
    (first, (a, b))
}

fn counted<A: Iterator>(pair: (A, u8)) -> (A, u8) {
    let (mut a, n) = pair;
    a.next();
    (a, n + 1)
}

trait Step {
    fn step(self) -> Self;
}

impl<A: Iterator, B: Iterator> Step for (A, B) {
    fn step(mut self) -> Self {
        self.0.next();
        self.1.next();
        self
    }
}

fn stepped<P: Step>(p: P) -> P {
    p.step()
}

trait Tools: Iterator {
    fn twice(mut self) -> (Option<Self::Item>, Self)
    where
        Self: Sized,
    {
        self.next();
        (self.next(), self)
    }
}

impl<T: Iterator> Tools for T {}

fn via<T: Tools>(t: T) -> (Option<T::Item>, T) {
    t.twice()
}

fn main() {
    let (first, (light, sevens)) = firsts((Light::Green, Sevens));
    println!("{first:?} {light:?} {sevens:?}");
    let pair = (Light::Red, Light::Red);
    let (again, (a, b)) = firsts(pair);
    println!("{again:?} {a:?} {b:?}");
    println!("{:?}", counted((Light::Green, 4)));
    let (light, sevens) = stepped((Light::Red, Sevens));
    println!("{light:?} {sevens:?}");
    println!("{:?} {:?}", Light::Red.twice(), via(Light::Green));
    let (seven, sevens) = via(Sevens);
    println!("{seven:?} {sevens:?}");
}
