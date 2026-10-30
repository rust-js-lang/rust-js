//@ library-differs
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

struct Wrapper(Vec<u32>);

impl Deref for Wrapper {
    type Target = Vec<u32>;
    fn deref(&self) -> &Vec<u32> {
        &self.0
    }
}

impl DerefMut for Wrapper {
    fn deref_mut(&mut self) -> &mut Vec<u32> {
        &mut self.0
    }
}

fn push_one<P: DerefMut<Target = Vec<u32>>>(mut p: P) -> P {
    p.push(1);
    p
}

fn grown<P: DerefMut<Target = Vec<u32>>>(mut p: P) -> usize {
    p.push(0);
    p.len()
}

fn bump<P: DerefMut<Target = u32>>(p: &mut P) {
    **p += 10;
}

fn shout<P: DerefMut<Target = String>>(p: &mut P) {
    p.make_ascii_uppercase();
}

fn main() {
    println!("{:?}", push_one(Wrapper(vec![7])).0);
    println!("{:?}", push_one(Box::new(vec![8])));
    let mut v = vec![9];
    push_one(&mut v);
    println!("{v:?}");
    println!("{} {}", grown(Box::new(vec![1, 2])), grown(&mut v));

    let mut b = Box::new(1u32);
    bump(&mut b);
    println!("{b}");
    let mut n = 2u32;
    let mut r = &mut n;
    bump(&mut r);
    println!("{n}");
    let cell = RefCell::new(3u32);
    bump(&mut cell.borrow_mut());
    println!("{}", cell.borrow());

    let mut s = Box::new(String::from("hi"));
    shout(&mut s);
    println!("{s}");

    either_sides();
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Deref, R: Deref<Target = L::Target>> Deref for Either<L, R> {
    type Target = L::Target;
    fn deref(&self) -> &L::Target {
        match self {
            Either::Left(l) => l,
            Either::Right(r) => r,
        }
    }
}

impl<L: DerefMut, R: DerefMut<Target = L::Target>> DerefMut for Either<L, R> {
    fn deref_mut(&mut self) -> &mut L::Target {
        match self {
            Either::Left(l) => l,
            Either::Right(r) => r,
        }
    }
}

fn either_sides() {
    let mut left: Either<Box<Vec<u32>>, Wrapper> = Either::Left(Box::new(vec![1]));
    left.push(2);
    *left = vec![3, 4];
    let mut right: Either<Box<Vec<u32>>, Wrapper> = Either::Right(Wrapper(vec![5]));
    right.push(6);
    println!("{:?} {:?}", *left, *right);
    println!("{}", push_one(right).len());
}
