// A `skip(n)`'s `last` of a `&mut dyn Iterator` asks its source's `nth(n -
// 1)`, then its `last`, as std's `Skip::last` does: a `&mut dyn` passes
// `nth` on to the crate's own, and `last` to std's default, of `next`. So
// does a `step_by`'s, an `enumerate`'s, and a `rev`'s of a `&mut dyn
// DoubleEndedIterator`, its `nth_back` (#2218).

struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn count(self) -> usize {
        println!("own count {}", self.0);
        self.0 as usize
    }
    fn last(self) -> Option<u32> {
        println!("own last {}", self.0);
        self.0.checked_sub(1).map(|_| 0)
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
    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

fn main() {
    println!("a {:?}", Own(3).skip(1).last());
    let mut own = Own(3);
    println!("b {:?}", (&mut own).skip(1).last());
    let mut own = Own(3);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("c {:?}", lent.skip(1).last());
    let mut own = Own(4);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("d {:?}", lent.skip(2).last());
    let mut own = Own(6);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("step_by {:?}", lent.step_by(2).last());
    let mut own = Own(5);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("enumerate {:?}", lent.enumerate().skip(1).last());
    let mut own = Own(5);
    let lent: &mut dyn DoubleEndedIterator<Item = u32> = &mut own;
    println!("rev {:?}", lent.rev().skip(1).last());
    let mut own = Own(5);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("past its end {:?}", lent.skip(9).last());
    println!("left {:?}", own.0);
}
