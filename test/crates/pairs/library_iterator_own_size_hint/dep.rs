//! A library's iterator with its own `size_hint`, and generic code that
//! asks it: what the program gives that code knows the library's own, as
//! the program has compiled it itself for its own call of it (#1023).
pub struct Countdown(pub u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0 as usize;
        (n, Some(n))
    }
}

pub fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}
