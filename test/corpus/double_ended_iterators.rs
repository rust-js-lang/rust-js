// An iterator of the crate's that runs from both ends and knows its length,
// as smallvec's, either's and memchr's do: its own `next_back` is what
// `rev()` and `next_back()` call, and its own `len` what `len()` is.

#[derive(Debug)]
struct Span {
    low: u32,
    high: u32,
}

impl Iterator for Span {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.low >= self.high {
            return None;
        }
        self.low += 1;
        Some(self.low - 1)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.high - self.low) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<u32> {
        if self.low >= self.high {
            return None;
        }
        self.high -= 1;
        println!("back {}", self.high);
        Some(self.high)
    }
}

impl ExactSizeIterator for Span {}

fn main() {
    let mut s = Span { low: 0, high: 5 };
    println!("{:?} {:?} {}", s.next(), s.next_back(), s.len());
    let backwards: Vec<u32> = Span { low: 1, high: 4 }.rev().collect();
    println!("{:?}", backwards);
    let mut both = Span { low: 10, high: 14 };
    println!("{:?} {:?} {:?}", both.next_back(), both.next(), both.len());
    for x in (Span { low: 0, high: 3 }).rev() {
        println!("got {x}");
    }
    println!("{:?}", Span { low: 0, high: 6 }.rev().take(2).collect::<Vec<_>>());
}
