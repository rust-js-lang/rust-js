// A `copied()` of the crate's iterator that writes `nth_back` steps it as
// std's `Copied` does: `Copied` doesn't write `nth_back`, so its default,
// its source's `advance_back_by` then `next_back`, never the crate's own,
// directly, through `rev()`, in generic code and of a clone of one; its
// `next_back` and `nth` pass on as before (#2130).
#[derive(Clone)]
struct Refs<'a>(&'a [u32]);
impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        println!("next");
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
}
impl<'a> DoubleEndedIterator for Refs<'a> {
    fn next_back(&mut self) -> Option<&'a u32> {
        println!("back");
        let (last, rest) = self.0.split_last()?;
        self.0 = rest;
        Some(last)
    }
    fn nth_back(&mut self, n: usize) -> Option<&'a u32> {
        println!("own nth_back {n}");
        let len = self.0.len();
        let item = len.checked_sub(n + 1).map(|i| &self.0[i]);
        self.0 = &self.0[..len.saturating_sub(n + 1)];
        item
    }
}

fn generic<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Option<u32> {
    i.copied().nth_back(1)
}

fn generic_rev<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Option<u32> {
    i.copied().rev().nth(1)
}

fn generic_clone<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> (Option<u32>, Option<u32>) {
    let mut copied = i.copied();
    let mut again = copied.clone();
    (copied.nth_back(1), again.nth_back(0))
}

fn generic_forward<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> (Option<u32>, Option<u32>) {
    let mut copied = i.copied();
    (copied.next_back(), copied.next())
}

fn main() {
    let items = [1, 2, 3, 4];
    println!("{:?}", Refs(&items).copied().nth_back(1));
    println!("{:?}", Refs(&items).copied().rev().nth(1));
    println!("{:?}", Refs(&items).copied().rev().skip(1).next());
    println!("{:?}", generic(Refs(&items)));
    println!("{:?}", generic_rev(Refs(&items)));
    println!("{:?}", generic_clone(Refs(&items)));
    println!("{:?}", generic_forward(Refs(&items)));
    println!("{:?}", generic(items.iter()));
    println!("{:?}", generic_clone(items.iter()));
}
