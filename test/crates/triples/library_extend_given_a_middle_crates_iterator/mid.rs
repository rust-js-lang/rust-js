//! An iterator with a destructor, which this crate's generic code extends
//! a collection with: the first library's `Bag` its consumer gives, whose
//! `for` loop drops it at its end.
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
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

pub fn up<C: Extend<u32>>(mut c: C) -> C {
    c.extend(Up { n: 0 });
    c
}
