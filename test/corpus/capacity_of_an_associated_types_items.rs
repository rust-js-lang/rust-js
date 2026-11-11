//@ library-differs
// A `Vec` of an associated type's items, `I::Item` or `S::Item`, or of a
// type built of type parameters, `(T, u8)`, or a reference to one, `&T`,
// grown by generic code: its capacity is std's, by the items' size its
// caller or its trait's dictionary gives, or rustc's where it knows it
// (#1260).
fn pushed<I: Iterator>(it: I) -> usize {
    let mut v: Vec<I::Item> = Vec::new();
    for x in it {
        v.push(x);
    }
    v.capacity()
}

fn extended<I: Iterator>(it: I) -> usize {
    let mut v: Vec<I::Item> = Vec::new();
    v.extend(it);
    v.capacity()
}

fn collected<I: Iterator>(it: I) -> usize {
    let v: Vec<I::Item> = it.collect();
    v.capacity()
}

fn reserved<I: ExactSizeIterator>(it: I) -> usize {
    let mut v: Vec<I::Item> = Vec::with_capacity(1);
    v.reserve(it.len() + 3);
    v.capacity()
}

fn paired<T: Copy>(x: T) -> usize {
    let mut v: Vec<(T, u8)> = Vec::new();
    v.push((x, 1));
    v.push((x, 2));
    v.capacity()
}

fn sized_and_grown<T: Copy>(x: T) -> (usize, usize) {
    let mut v: Vec<T> = Vec::new();
    v.push(x);
    (std::mem::size_of::<T>(), v.capacity())
}

fn referenced<T>(x: &T) -> usize {
    let mut v: Vec<&T> = Vec::new();
    v.push(x);
    v.capacity()
}

trait Source {
    type Item;
    fn items(&self) -> Vec<Self::Item>;
}

struct Bytes;
impl Source for Bytes {
    type Item = u8;
    fn items(&self) -> Vec<u8> {
        vec![1, 2, 3]
    }
}

struct Wide;
impl Source for Wide {
    type Item = [u64; 4];
    fn items(&self) -> Vec<[u64; 4]> {
        vec![[0; 4]]
    }
}

fn gathered<S: Source>(s: &S) -> usize {
    let mut all: Vec<S::Item> = Vec::new();
    for _ in 0..3 {
        all.extend(s.items());
    }
    all.push(s.items().pop().unwrap());
    all.capacity()
}

fn main() {
    println!("{}", pushed([1u8, 2, 3].into_iter()));
    println!("{}", pushed([[1u64; 4], [2; 4]].into_iter()));
    println!("{}", pushed((0..5).map(|_| ())) == usize::MAX);
    println!("{}", extended([1u8, 2, 3].into_iter().filter(|x| *x > 1)));
    println!("{}", extended(vec![[1u64; 4]; 3].into_iter()));
    println!("{}", collected([1u16, 2, 3].into_iter().filter(|x| *x > 0)));
    println!("{}", reserved([1u8, 2].into_iter()));
    println!("{}", reserved([[1u64; 4]].into_iter()));
    println!("{} {}", gathered(&Bytes), gathered(&Wide));
    println!("{} {}", paired(1u8), paired([0u64; 4]));
    println!("{} {}", referenced(&1u8), referenced(&[0u64; 4]));
    println!("{:?} {:?}", sized_and_grown(1u8), sized_and_grown([0u64; 4]));
}
