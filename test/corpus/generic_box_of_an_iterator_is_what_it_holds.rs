// Generic code's sized `Box<I>` and `&mut Box<I>` of the crate's iterator
// are what they hold for every question of the iterator (#2219, #2221):
// given its dictionary through the `&mut` and the box, so it's stepped,
// counted and summed where it was never stepped or threw; asked its hint
// where a collect asks it; closed to drop it where it leaked; its own
// `nth` reached through a `skip`. A `&mut I`'s flatten and chain close
// what they hold, never the lent iterator.
use std::collections::HashSet;
use std::iter::Sum;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Total(u32);

impl Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

impl ExactSizeIterator for Own {}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own {}", self.0);
    }
}

fn skipped_count<I: Iterator<Item = u32>>(it: &mut Box<I>) -> usize {
    it.skip(1).count()
}

fn stepped<I: Iterator<Item = u32>>(it: &mut Box<I>) -> Option<u32> {
    it.next()
}

fn summed<I: Iterator<Item = u32>>(it: &mut Box<I>) -> u32 {
    it.sum()
}

fn enumerated<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(it: &mut Box<I>) -> Vec<u32> {
    it.skip(1).take(2).collect()
}

fn hinted<I: Iterator<Item = u32>>(it: Box<I>) -> usize {
    it.step_by(2).collect::<HashSet<_>>().len()
}

fn zipped<I: Iterator<Item = u32>>(it: Box<I>) -> Option<(u32, u32)> {
    let mut zip = it.zip(10..13u32);
    zip.nth(1)
}

fn flattened<I: Iterator<Item = u32>>(it: &mut I) -> u32 {
    let total: Total = it.flat_map(|x| vec![D(x + 10), D(x)]).sum();
    total.0
}

fn chained<I: Iterator<Item = u32>>(it: &mut I) -> Result<u32, u32> {
    it.map(D).chain(vec![D(6), D(7)]).try_fold(0, |n, d| if n > 3 { Err(n) } else { Ok(n + d.0) })
}

fn main() {
    let mut b = Box::new(Own(5));
    println!("a {}", skipped_count(&mut b));
    println!("a {:?}", b.next());
    let mut b = Box::new(Own(5));
    println!("b {:?}", stepped(&mut b));
    println!("c {}", summed(&mut b));
    let mut b = Box::new(Own(5));
    println!("d {:?}", enumerated(&mut b));
    println!("e {}", hinted(Box::new(Own(5))));
    println!("f {:?}", zipped(Box::new(Own(5))));
    let mut o = Own(3);
    println!("h {}", flattened(&mut o));
    let mut o = Own(5);
    println!("i {:?}", chained(&mut o));
    println!("after");
}
