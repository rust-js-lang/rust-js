//! A consumer giving the library's default and its function a unit
//! struct's iterator, `undefined` in JS, and a tuple struct's, `[0]`, each
//! as it is with its dictionary (itertools' laziness tests).
use dep::Tools;

/// Gives what's after its number up to 3.
struct Three(u8);

impl Iterator for Three {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        (self.0 < 3).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

/// Gives none, a unit struct.
struct Never;

impl Iterator for Never {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        None
    }
}

pub fn main() {
    println!("{:?}", Three(1).then_all(Three(0)).collect::<Vec<_>>());
    println!("{:?}", Never.then_all(Three(2)).collect::<Vec<_>>());
    println!("{:?}", Three(0).then_all(Never).collect::<Vec<_>>());
    println!("{:?}", dep::paired(Never, Three(1)).collect::<Vec<_>>());
    println!("{:?}", dep::paired(Three(2), vec![5, 6]).collect::<Vec<_>>());
    println!("{:?} {:?}", Never.firsts(), Three(0).firsts());
}
