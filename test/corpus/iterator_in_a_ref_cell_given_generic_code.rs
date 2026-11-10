//@ library-differs
// An iterator a `RefCell` holds, of a type only a caller knows, stepped
// through its guard: `(*items.borrow_mut()).next()` of an `I: Iterator`,
// or an associated type's, `S::Iter`, and given on, `&mut *held`, to
// generic code that steps it. A `&mut` to such an `I` is the iterator
// itself (`mut_ref_form`), so the guard derefs to its cell's `value`, where
// it gave the cell, which threw `it.next is not a function` (#1016); and
// std's `&mut I` isn't made a JS iterator of, `Iterator.from(..)`, but is
// the place, which `*held.borrow_mut() = other` writes, where it crashed.
// A closure's, `F: FnMut`, is called through the guard as it was.
use std::cell::RefCell;
use std::rc::Rc;

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
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|x| 10 + x)
    }
}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop countdown at {}", self.0);
    }
}

trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(&self) -> Self::Iter;
}

struct Counted(u32);

impl Source for Counted {
    type Iter = Countdown;
    fn items(&self) -> Countdown {
        Countdown(self.0)
    }
}

struct Listed(Vec<u32>);

impl Source for Listed {
    type Iter = std::vec::IntoIter<u32>;
    fn items(&self) -> Self::Iter {
        self.0.clone().into_iter()
    }
}

fn step<I: Iterator<Item = u32>>(it: &mut I) -> Option<u32> {
    it.next()
}

fn first<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    let items = Rc::new(RefCell::new(it));
    (*items.borrow_mut()).next()
}

fn second<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    let items = RefCell::new(it);
    let mut held = items.borrow_mut();
    held.next();
    step(&mut *held)
}

fn summed<I: Iterator<Item = u32>>(it: I) -> u32 {
    let items = RefCell::new(it);
    items.borrow_mut().by_ref().sum()
}

fn last<S: Source>(source: &S) -> Option<u32> {
    let items = Rc::new(RefCell::new(source.items()));
    items.borrow_mut().next_back()
}

fn second_of<S: Source>(source: &S) -> Option<u32> {
    let items = Rc::new(RefCell::new(source.items()));
    step(&mut *items.borrow_mut());
    (*items.borrow_mut()).next()
}

fn replaced<I: Iterator<Item = u32>>(it: I, other: I) -> Vec<u32> {
    let held = RefCell::new(it);
    *held.borrow_mut() = other;
    held.borrow_mut().next();
    held.into_inner().collect()
}

fn through<F: FnMut() -> u32>(f: &mut F) -> u32 {
    f()
}

fn called<F: FnMut() -> u32>(f: F) -> u32 {
    let held = Rc::new(RefCell::new(f));
    let first = through(&mut *held.borrow_mut());
    first * 10 + (*held.borrow_mut())()
}

fn main() {
    println!("{:?}", first(Countdown(3)));
    println!("{:?}", first(vec![4, 5].into_iter()));
    println!("{:?}", second(Countdown(3)));
    println!("{:?}", second(vec![4, 5, 6].into_iter()));
    println!("{}", summed(vec![4, 5, 6].into_iter()));
    println!("{:?}", last(&Counted(3)));
    println!("{:?}", last(&Listed(vec![4, 5, 6])));
    println!("{:?}", second_of(&Counted(4)));
    println!("{:?}", second_of(&Listed(vec![7, 8])));
    println!("{:?}", replaced(vec![1, 2].into_iter(), vec![7, 8, 9].into_iter()));
    let mut n = 0;
    println!(
        "{}",
        called(|| {
            n += 1;
            n
        })
    );
}
