// `copied()` and `cloned()` of the crate's iterator ask it what std's
// `Copied` and `Cloned` do: `Copied` writes `count` and `last`, its
// source's own; `Cloned` writes neither, so its `count` and `last` are
// std's defaults, its `fold`, which steps the crate's `next` (#2122).

struct Refs<'a>(&'a [u32]);
impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        println!("next");
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
    fn count(self) -> usize {
        println!("own count");
        self.0.len()
    }
    fn last(self) -> Option<&'a u32> {
        println!("own last");
        self.0.last()
    }
}
fn main() {
    let items = [1, 2, 3];
    println!("cloned count {}", Refs(&items).cloned().count());
    println!("cloned last {:?}", Refs(&items).cloned().last());
    println!("copied count {}", Refs(&items).copied().count());
    println!("copied last {:?}", Refs(&items).copied().last());
    println!("skip cloned count {}", Refs(&items).cloned().skip(1).count());
}
