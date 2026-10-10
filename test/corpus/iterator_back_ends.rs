// An iterator's back: `rev()` of a range, an inclusive one, items made
// already and an iterator of the crate's own that runs from both ends;
// `next_back()` and `len()` of one that knows where it is, and a
// `Peekable`'s `peek`, `next_if` and `next_if_eq`.
struct Span(u32, u32);

impl Iterator for Span {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0 - 1)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.1 -= 1;
            Some(self.1)
        } else {
            None
        }
    }
}

fn main() {
    let down: Vec<u32> = (1..4).rev().collect();
    let top: Vec<u8> = (250..=255u8).rev().collect();
    let own: Vec<u32> = Span(0, 3).rev().collect();
    println!("{down:?} {top:?} {own:?}");

    let v = vec![10, 20, 30, 40];
    let mut it = v.iter();
    let back = it.next_back();
    let front = it.next();
    println!("{back:?} {front:?} {} {:?}", it.len(), it.as_slice());

    let mut range = 0..5;
    println!("{:?} {:?}", range.next_back(), range.next());

    let mut p = v.iter().peekable();
    let peeked = p.peek().copied();
    let ten = p.next_if_eq(&&10);
    let big = p.next_if(|x| **x > 100);
    println!("{peeked:?} {ten:?} {big:?} {:?}", p.next());
}
