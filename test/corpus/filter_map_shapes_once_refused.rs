// A filter_map's shapes rust-js refused before the filter_map family's
// flip, each std's compiled `FilterMap` now and run as native Rust runs it
// (#1681's change 2): the hint of one generic code makes of a type
// parameter's, a held `rev()` of one of a `skip(1)` its hint asked
// (#1427), and a `peekable()` of one made here its hint asked (#382).
pub fn generic_hint<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    it.filter_map(|x| x.checked_sub(1)).size_hint()
}

pub fn rev_of_filter_map_of_skip() -> (usize, Option<usize>) {
    let it = vec![1u8, 2, 3].into_iter().skip(1).filter_map(|x| if x > 1 { Some(x) } else { None }).rev();
    it.size_hint()
}

pub fn peekable_filter_map(v: &[u32]) -> (usize, Option<usize>) {
    v.iter().filter_map(|x| if *x > 1 { Some(*x) } else { None }).peekable().size_hint()
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

fn main() {
    println!("{:?}", generic_hint(vec![1u32, 0, 3].into_iter()));
    println!("{:?}", generic_hint(0..5u32));
    println!("{:?}", generic_hint(Countdown(4)));
    println!("{:?}", generic_hint(std::iter::empty()));
    println!("{:?}", rev_of_filter_map_of_skip());
    println!("{:?}", peekable_filter_map(&[1, 2, 3]));
    println!("{:?}", peekable_filter_map(&[]));
}
