//! A consumer of a library's trait whose associated type is an iterator:
//! the library's impl and one of its own, a crate's iterator, through the
//! library's generic code, each given back as its own type (#2330).
use dep::{Ranged, Source, from_its_back, given_back};

#[derive(Debug)]
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

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|x| 10 + x)
    }
}

struct Counted;

impl Source for Counted {
    type Out = Countdown;
    fn make(&self, n: u32) -> Countdown {
        Countdown(n)
    }
}

pub fn main() {
    let r = given_back(&Ranged, 3);
    println!("{:?} {} {:?}", r, r.start, r.clone().collect::<Vec<_>>());
    let c = given_back(&Counted, 3);
    println!("{:?} {}", c, c.0);
    println!("{:?} {:?}", from_its_back(&Ranged, 3), from_its_back(&Counted, 3));
}
