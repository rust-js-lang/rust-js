// A `cloned()` of what generic code is given, cloned, then stepped from
// its back: its clone is made of a clone of what the `cloned()` is made of,
// so that is made knowing its back where it's made, as a `copied()`'s is,
// though the `cloned()` itself is never stepped from its back (#2137).

#[derive(Clone)]
struct Refs<'a>(&'a [u32]);

impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
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

fn next_back<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> Option<u32> {
    i.cloned().clone().next_back()
}

fn nth_back<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> Option<u32> {
    i.cloned().clone().nth_back(1)
}

fn rfold<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> u32 {
    i.cloned().clone().rfold(0, |a, x| a * 2 + x)
}

fn rev_nth<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> Option<u32> {
    i.cloned().clone().rev().nth(1)
}

fn held<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(i: I) -> Option<u32> {
    let cloned = i.cloned();
    let mut again = cloned.clone();
    drop(cloned);
    again.nth_back(1)
}

fn main() {
    let items = [1, 2, 3, 4];
    println!("{:?}", next_back(Refs(&items)));
    println!("{:?}", nth_back(Refs(&items)));
    println!("{:?}", rfold(Refs(&items)));
    println!("{:?}", rev_nth(Refs(&items)));
    println!("{:?}", held(Refs(&items)));
    println!("{:?}", next_back(items.iter()));
    println!("{:?}", nth_back(items.iter()));
    println!("{:?}", held(items.iter()));
}
