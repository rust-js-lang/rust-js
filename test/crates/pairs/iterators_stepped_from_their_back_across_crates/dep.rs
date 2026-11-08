//! Iterators a consumer steps from their back where this library can't see
//! (#1024): what a public function gives behind an `impl
//! DoubleEndedIterator`, a filter of the crate's own iterator or of a
//! slice's items, made knowing its back where it's made; and a generic
//! function given a std adapter of a double-ended source, which steps it
//! from its back, its consumer's adapter made knowing it too.
pub struct Down {
    pub n: u32,
}

impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            self.n
        })
    }
}

impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            100 + self.n
        })
    }
}

pub fn filtered() -> impl DoubleEndedIterator<Item = u32> {
    Down { n: 4 }.filter(|x| x % 2 == 0)
}

pub struct Bag(pub Vec<i32>);

impl Bag {
    pub fn odd_squares(&self) -> impl DoubleEndedIterator<Item = i32> + '_ {
        self.0.iter().filter(|x| *x % 2 == 1).map(|x| x * x)
    }

    pub fn evens(&self) -> impl DoubleEndedIterator<Item = &i32> + '_ {
        self.0.iter().filter(|x| *x % 2 == 0)
    }
}

pub fn back<I, P>(mut it: std::iter::Filter<I, P>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = u32>,
    P: FnMut(&u32) -> bool,
{
    (it.next_back(), it.next())
}

pub fn last_mapped<I, F>(mut it: std::iter::Map<I, F>) -> Option<u32>
where
    I: DoubleEndedIterator<Item = u32>,
    F: FnMut(u32) -> u32,
{
    it.next_back()
}
