// `self.borrow()` and `self.borrow_mut()` of a trait whose supertrait is
// `BorrowMut<[usize]>`, as itertools' `PoolIndex` is: its dictionary's, its
// `BorrowMut` entry's, and that one's `Borrow` entry's, std's of a `Vec` and
// an array the same array (ADR 0483).
use std::borrow::BorrowMut;

pub trait PoolIndex: BorrowMut<[usize]> {
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

// A type of the crate's own, borrowed as part of what it holds: generic
// code's `p.borrow()` of a `P: BorrowMut<[usize]>` is its `BorrowMut`
// dictionary's `Borrow` entry, the crate's impl.
struct Tail {
    items: Vec<usize>,
}

impl std::borrow::Borrow<[usize]> for Tail {
    fn borrow(&self) -> &[usize] {
        &self.items[1..]
    }
}

impl BorrowMut<[usize]> for Tail {
    fn borrow_mut(&mut self) -> &mut [usize] {
        &mut self.items[1..]
    }
}

fn bump<P: BorrowMut<[usize]>>(mut p: P) -> usize {
    p.borrow_mut()[0] += 1;
    let all: &[usize] = p.borrow();
    all.iter().sum()
}

fn main() {
    println!("{}", vec![1, 2, 3].len());
    println!("{}", PoolIndex::len(&vec![1usize, 2]));
    println!("{}", total(vec![5usize, 6]));
    println!("{}", total([7usize, 8, 9]));
    let tail = Tail {
        items: vec![10, 20, 30],
    };
    println!("{} {}", bump(vec![1usize, 2]), bump(tail));
}
