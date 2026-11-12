//! A library's own `Extend`, called by its generic code through a
//! dictionary, given an iterator with a destructor: its `for` loop drops it
//! at its end, as natively (#1591), though none of its consumer's own
//! values drops.
pub struct Up {
    pub n: u32,
}

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("fold");
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

pub struct Bag(pub Vec<u32>);

impl Extend<u32> for Bag {
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        for x in iter {
            self.0.push(x);
        }
    }
}

pub fn up<C: Extend<u32>>(mut c: C) -> C {
    c.extend(Up { n: 0 });
    c
}

pub fn given<C: Extend<u32>, I: IntoIterator<Item = u32>>(mut c: C, items: I) -> C {
    c.extend(items);
    c
}
