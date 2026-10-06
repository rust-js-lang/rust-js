// A trait's default methods, copied into each impl (ADR 0049), each decide
// what a clone copies under their own bounds: `a`'s `T::Item` is a number,
// which a clone can share, and `b`'s a `Vec`, which a clone must copy,
// though both clone a `Holder<T>`.
#[derive(Clone)]
struct Holder<T: Iterator> {
    item: T::Item,
}

trait Tr {
    fn a<T: Iterator<Item = u32> + Clone>(h: &Holder<T>) -> Holder<T> {
        h.clone()
    }
    fn b<T: Iterator<Item = Vec<u32>> + Clone>(h: &Holder<T>) -> Holder<T> {
        h.clone()
    }
}

#[derive(Clone)]
struct Numbers;

impl Iterator for Numbers {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        None
    }
}

#[derive(Clone)]
struct Lists;

impl Iterator for Lists {
    type Item = Vec<u32>;
    fn next(&mut self) -> Option<Vec<u32>> {
        None
    }
}

struct S;

impl Tr for S {}

fn main() {
    let n = Holder::<Numbers> { item: 1 };
    let c = S::a(&n);
    println!("{} {}", n.item, c.item);

    let h = Holder::<Lists> { item: vec![1] };
    let mut c = S::b(&h);
    c.item.push(2);
    println!("{:?} {:?}", h.item, c.item);
}
