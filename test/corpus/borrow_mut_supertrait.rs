// `self.borrow()` and `self.borrow_mut()` of a trait whose supertrait is
// `BorrowMut<[usize]>`, as itertools' `PoolIndex` is: its dictionary's, its
// `BorrowMut` entry's, and that one's `Borrow` entry's, std's of a `Vec` and
// an array the same array (ADR 0483).
use std::borrow::BorrowMut;

trait PoolIndex: BorrowMut<[usize]> {
    fn len(&self) -> usize {
        self.borrow().len()
    }

    fn first(&mut self) -> usize {
        self.borrow_mut()[0] += 1;
        self.borrow()[0]
    }
}

impl PoolIndex for Vec<usize> {}

impl<const K: usize> PoolIndex for [usize; K] {}

fn total<P: PoolIndex>(mut p: P) -> usize {
    p.first() + p.len()
}

fn main() {
    println!("{}", vec![1, 2, 3].len());
    println!("{}", PoolIndex::len(&vec![1usize, 2]));
    println!("{}", total(vec![5usize, 6]));
    println!("{}", total([7usize, 8, 9]));
}
