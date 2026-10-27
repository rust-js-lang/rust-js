// A flatten stepped from both ends, as std's `Flatten` steps it: each end
// takes from the inner list the other started once its source is done, and
// its source, fused, isn't stepped again past a `None` (#382).

// Gives `None` once after its first list, then more: what a `Fuse` keeps done.
struct Flaky(u32);
impl Iterator for Flaky {
    type Item = Vec<u32>;
    fn next(&mut self) -> Option<Vec<u32>> {
        self.0 += 1;
        match self.0 {
            1 => Some(vec![1, 2]),
            2 => None,
            3 => Some(vec![3]),
            _ => None,
        }
    }
}
impl DoubleEndedIterator for Flaky {
    fn next_back(&mut self) -> Option<Vec<u32>> {
        self.0 += 1;
        match self.0 {
            1 => Some(vec![8, 9]),
            2 => None,
            3 => Some(vec![7]),
            _ => None,
        }
    }
}
fn both<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> [Option<u32>; 4] {
    [it.next(), it.next_back(), it.next_back(), it.next()]
}

fn main() {
    let v = vec![vec![1, 2, 3], vec![4], vec![5, 6]];
    let mut f = v.iter().flatten();
    println!("{:?} {:?} {:?} {:?}", f.next(), f.next_back(), f.next(), f.next_back());
    println!("{:?} {:?} {:?}", f.next(), f.next_back(), f.next());
    let mut g = vec![vec![1, 2], vec![3, 4]].into_iter().flatten();
    println!(
        "{:?} {:?} {:?} {:?} {:?}",
        g.next_back(),
        g.next(),
        g.next(),
        g.next(),
        g.next_back()
    );
    let one = vec![vec![1, 2, 3]];
    let mut m = one.iter().flatten();
    println!(
        "{:?} {:?} {:?} {:?}",
        m.next(),
        m.next_back(),
        m.next_back(),
        m.next_back()
    );
    let mut h = Flaky(0).flatten();
    println!(
        "{:?} {:?} {:?} {:?} {:?}",
        h.next(),
        h.next(),
        h.next(),
        h.next(),
        h.next()
    );
    let mut k = Flaky(0).flatten();
    println!("{:?} {:?} {:?} {:?}", k.next(), k.next_back(), k.next_back(), k.next());
    println!("{:?}", both(Flaky(0).flatten()));
    println!("{:?}", both(vec![vec![1, 2, 3]].into_iter().flatten()));
}
