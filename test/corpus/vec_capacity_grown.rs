//@ native: wasm32
// A `Vec`'s capacity, in a crate that reads one, as std's growth gives it
// (#1180, ADR 0419 amended): `push` and `insert` grow it by
// `grow_amortized`, at least twice what it had and `MIN_NON_ZERO_CAP`, 8 of
// 1-byte items, 4 up to 1 KiB, 1 above; `append`, `resize`,
// `extend_from_slice` and an `extend` of a `TrustedLen` iterator reserve as
// many as they add; a `collect()` or an `extend` of another grows by its
// hint's lower bound as it fills, and one into a `Result` by the
// `GenericShunt`'s, none. A `VecDeque`'s and a `BinaryHeap`'s alike, and of
// a type parameter's items, by its caller's size.
use std::collections::{BinaryHeap, VecDeque};

fn pushes<T: Clone>(x: T, n: usize) -> Vec<usize> {
    let mut v = Vec::new();
    let mut seen = Vec::new();
    for _ in 0..n {
        v.push(x.clone());
        seen.push(v.capacity());
    }
    seen
}

fn kept<T: PartialEq + Clone>(xs: &[T], not: &T) -> usize {
    let v: Vec<T> = xs.iter().filter(|x| *x != not).cloned().collect();
    v.capacity()
}

fn grown<T: Clone>(v: &mut Vec<T>, x: &T, n: usize) -> usize {
    v.extend(std::iter::repeat(x.clone()).take(n));
    v.extend((0..n).map(|_| x.clone()).filter(|_| true));
    v.insert(0, x.clone());
    v.capacity()
}

fn fill<C: Extend<u32>, I: IntoIterator<Item = u32>>(c: &mut C, items: I) {
    c.extend(items);
}

fn parsed(xs: &[&str]) -> Result<Vec<u32>, std::num::ParseIntError> {
    xs.iter().map(|x| x.parse::<u32>()).collect()
}

fn somes<T: Copy>(xs: &[T]) -> Option<Vec<T>> {
    xs.iter().map(|x| Some(*x)).collect()
}

fn main() {
    println!("{:?}", pushes(1u8, 10));
    println!("{:?}", pushes(1u16, 10));
    println!("{:?}", pushes(1u64, 10));
    println!("{:?}", pushes([0u8; 2000], 4));
    println!("{:?}", pushes((), 3));

    let mut w = vec![1u64, 2, 3];
    w.insert(0, 9);
    w.extend_from_slice(&[4, 5, 6, 7]);
    println!("{} {}", w.len(), w.capacity());
    let mut a = vec![1u32];
    let mut o = vec![2u32; 5];
    a.append(&mut o);
    println!("{} {} {}", a.capacity(), o.len(), o.capacity());
    let mut r = vec![0u16; 3];
    r.resize(10, 1);
    print!("{} ", r.capacity());
    r.resize(2, 1);
    println!("{}", r.capacity());

    let mut e: Vec<u32> = Vec::new();
    e.extend([1, 2, 3]);
    print!("{} ", e.capacity());
    e.extend(&[4, 5]);
    print!("{} ", e.capacity());
    e.extend((0..10).filter(|x| x % 3 == 0));
    print!("{} ", e.capacity());
    e.extend(vec![7u32; 9]);
    println!("{}", e.capacity());
    // Grown item by item, `4` then `8`, where reserving all five it adds
    // first would give `5`.
    let mut f: Vec<u32> = Vec::new();
    f.extend((0..10).filter(|x| x % 2 == 0));
    println!("{}", f.capacity());
    let mut s = vec![String::new()];
    s.extend([String::from("t"), String::from("u")].iter().cloned());
    println!("{}", s.capacity());

    let bytes: Vec<u8> = (0..20u8).filter(|x| x % 2 == 0).collect();
    let halves: Vec<u16> = [1u16, 2, 3, 4, 5].iter().filter(|x| *x % 2 == 1).cloned().collect();
    let words: Vec<u64> = (0u64..3).filter(|n| n % 2 == 0).collect();
    let big: Vec<[u8; 2000]> = (0..3).map(|_| [0u8; 2000]).filter(|_| true).collect();
    println!("{} {} {} {}", bytes.capacity(), halves.capacity(), words.capacity(), big.capacity());

    println!("{} {}", kept(&[1u64, 2, 3], &2), kept(&[1u8, 2, 3, 4, 5, 6, 7, 8, 9], &0));
    let mut g: Vec<u16> = Vec::new();
    print!("{} ", grown(&mut g, &3, 3));
    println!("{}", grown(&mut g, &3, 9));
    let mut b: Vec<u32> = Vec::with_capacity(3);
    fill(&mut b, (0..5).filter(|_| true));
    print!("{} ", b.capacity());
    fill(&mut b, [1, 2, 3, 4]);
    print!("{} ", b.capacity());
    let mut z: Vec<u32> = Vec::new();
    fill(&mut z, [1u32; 9]);
    println!("{}", z.capacity());
    println!("{}", parsed(&["1", "2", "3", "4", "5"]).unwrap().capacity());
    println!("{} {}", somes(&[1u8, 2, 3]).unwrap().capacity(), somes(&[1u64, 2, 3, 4, 5, 6]).unwrap().capacity());

    let mut d: VecDeque<u64> = VecDeque::new();
    d.extend((0..7).filter(|x| x % 2 == 0));
    print!("{} ", d.capacity());
    d.extend([1, 2, 3]);
    print!("{} ", d.capacity());
    d.push_front(9);
    d.push_front(9);
    println!("{}", d.capacity());
    let mut h = BinaryHeap::new();
    h.push(1u32);
    let mut other = BinaryHeap::from(vec![5u32, 6, 7, 8, 9]);
    h.append(&mut other);
    println!("{} {}", h.capacity(), other.capacity());
}
