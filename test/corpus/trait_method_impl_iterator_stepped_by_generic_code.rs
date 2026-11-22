//@ library-differs
// A trait's method giving an `impl Iterator`, rustc's associated type of it,
// made a JS iterator by its impl's dictionary entry and stepped by generic
// code's `for`: of a range, the crate's own iterator, std's adapter over a
// range an impl's type holds, and the impl's own type parameter held as it
// is, stepped by its `Iterator` dictionary, and from its back by its
// `DoubleEndedIterator`'s (ADR 0061's #2005 amendment, #2330, #2342).
trait Source {
    fn make(&self, n: u32) -> impl Iterator<Item = u32>;
}

struct Ranged;

impl Source for Ranged {
    fn make(&self, n: u32) -> impl Iterator<Item = u32> {
        0..n
    }
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

struct Counted;

impl Source for Counted {
    fn make(&self, n: u32) -> impl Iterator<Item = u32> {
        Countdown(n)
    }
}

#[derive(Clone)]
struct Wrapped<I>(I);

impl<I: Iterator<Item = u32> + Clone> Source for Wrapped<I> {
    fn make(&self, n: u32) -> impl Iterator<Item = u32> {
        self.0.clone().take(n as usize)
    }
}

struct Given<I>(I);

impl<I: Iterator<Item = u32> + Clone> Source for Given<I> {
    fn make(&self, _n: u32) -> impl Iterator<Item = u32> {
        self.0.clone()
    }
}

trait Ends {
    fn ends(&self) -> impl DoubleEndedIterator<Item = u32>;
}

impl<I: DoubleEndedIterator<Item = u32> + Clone> Ends for Given<I> {
    fn ends(&self) -> impl DoubleEndedIterator<Item = u32> {
        self.0.clone()
    }
}

fn last_first<E: Ends>(e: &E) -> (Option<u32>, Option<u32>) {
    let mut it = e.ends();
    (it.next_back(), it.next())
}

fn total<S: Source>(s: &S, n: u32) -> u32 {
    let mut t = 0;
    for x in s.make(n) {
        t += x;
    }
    t
}

fn main() {
    println!("{}", total(&Ranged, 4));
    println!("{}", total(&Counted, 5));
    println!("{}", total(&Wrapped(10..20), 3));
    println!("{}", total(&Given(5..8), 0));
    println!("{:?}", last_first(&Given(1..4)));
    println!("{:?}", last_first(&Given(vec![7, 8, 9].into_iter())));
}
