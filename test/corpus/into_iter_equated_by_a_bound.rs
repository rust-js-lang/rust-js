// An `IntoIterator` whose `IntoIter` a bound equates to a type, `J:
// IntoIterator<IntoIter = I>`, makes that type, which the body holds as it
// holds any value of it (#2520): the crate's iterator of a field, stepped by
// its own `next`, a fieldless enum's and a unit struct's in a place of their
// own (#2255), a `Range` read by its fields, a `vec::IntoIter` read as its
// slice; given by its caller, or by generic code that has it of its own
// bound. And a unit struct whose `IntoIter` is an iterator JS holds as no
// object, given generic code that gives that iterator back, `J::IntoIter`.

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

#[derive(Debug, Clone)]
struct Bin(Vec<u8>);

impl IntoIterator for Bin {
    type Item = u8;
    type IntoIter = std::vec::IntoIter<u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn pick<I: Iterator, J: IntoIterator<IntoIter = I>>(first: I, more: J) -> (I, I) {
    (first, more.into_iter())
}

fn step<I: Iterator, J: IntoIterator<IntoIter = I>>(more: J) -> I {
    let mut i = more.into_iter();
    i.next();
    i
}

fn total<I: Iterator<Item = u8>, J: IntoIterator<IntoIter = I>>(more: J) -> u32 {
    more.into_iter().take(3).map(u32::from).sum()
}

fn unbound<I, J: IntoIterator<IntoIter = I>>(more: J) -> I {
    more.into_iter()
}

fn rest<J: IntoIterator<IntoIter = std::vec::IntoIter<u8>>>(j: J) -> Vec<u8> {
    let mut it = j.into_iter();
    it.next();
    it.as_slice().to_vec()
}

fn span<J: IntoIterator<IntoIter = std::ops::Range<u32>>>(j: J) -> (u32, u32) {
    let r = j.into_iter();
    (r.start, r.end)
}

fn counted<I: Iterator + Clone, K: IntoIterator<IntoIter = I>>(k: K) -> (I, usize) {
    let s = step(k);
    let n = s.clone().take(4).count();
    (s, n)
}

fn via<K: IntoIterator<Item = u8>>(k: K) -> Option<u8> {
    step(k).next()
}

fn made<J: IntoIterator>(j: J) -> J::IntoIter {
    j.into_iter()
}

fn main() {
    let (mut a, mut b) = pick(Light::Green, Light::Red);
    println!("pick {:?} {:?}", a.next(), b.next());
    println!("pick {:?} {:?} {a:?} {b:?}", a.next(), b.next());
    let (mut a, mut b) = pick(Counter { n: 1 }, Counter { n: 5 });
    println!("pick {:?} {:?} {} {}", a.next(), b.next(), a.n, b.n);
    let (mut a, mut b) = pick(vec![1u8, 2].into_iter(), vec![7u8, 8]);
    println!("pick {:?} {:?} {:?}", a.next(), b.next(), b.as_slice());

    let mut s = step(Light::Red);
    println!("step {:?} {s:?}", s.next());
    let mut s = step(Counter { n: 3 });
    println!("step {:?} {}", s.next(), s.n);
    let mut s = step(vec![1u8, 2, 3]);
    println!("step {:?} {:?}", s.next(), s.as_slice());

    println!("total {} {}", total(Light::Green), total(Counter { n: 0 }));

    let mut a = unbound(Counter { n: 3 });
    let mut b = unbound(Light::Red);
    let mut c = unbound(vec![4u8, 5, 6]);
    println!("unbound {:?} {} {:?} {b:?} {:?}", a.next(), a.n, b.next(), c.next());

    println!(
        "rest {:?} {:?} {:?}",
        rest(vec![1u8, 2, 3]),
        rest(Bin(vec![4, 5, 6])),
        rest(vec![9u8].into_iter())
    );
    println!("span {:?} {:?}", span(2..7), span(0..1));

    let (mut a, n) = counted(Counter { n: 250 });
    let (mut b, m) = counted(vec![1u8, 2, 3]);
    let (mut c, _) = counted(Light::Green);
    println!("counted {:?} {n} {:?} {m} {:?}", a.next(), b.next(), c.next());

    println!(
        "via {:?} {:?} {:?} {:?}",
        via(vec![1u8, 2]),
        via(Light::Red),
        via(Counter { n: 7 }),
        via(Bag)
    );

    let mut e = made(Bag);
    println!("made {:?} {:?}", e.next(), e.size_hint());
    let mut l = made(Light::Green);
    println!("made {:?} {l:?}", l.next());
}
