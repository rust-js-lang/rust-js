// `slice::from_mut(&mut x)` and `array::from_mut(&mut x)` of a number, text or
// `Option`'s place (#1336): a view of the one item, whose every read and
// write is of `x`. Of an object, whose place `s[0] = other` would replace, it's
// refused.
use std::slice;

fn bump(s: &mut [i32]) {
    for x in s.iter_mut() {
        *x += 1;
    }
    s[0] *= 2;
}

fn append(s: &mut [String], text: &str) {
    s[0].push_str(text);
}

fn swap_in(s: &mut [u8], new: u8) -> u8 {
    std::mem::replace(&mut s[0], new)
}

fn double<T: Copy + std::ops::Add<Output = T>>(item: &mut T) {
    let s = slice::from_mut(item);
    s[0] = s[0] + s[0];
}

fn main() {
    let mut y = 5;
    bump(slice::from_mut(&mut y));
    println!("{y}");

    let mut t = String::from("a");
    append(slice::from_mut(&mut t), "bc");
    slice::from_mut(&mut t)[0].push('d');
    println!("{t}");

    let mut z = 1u8;
    let s = slice::from_mut(&mut z);
    s[0] = 9;
    println!("{} {:?} {}", s.len(), s, s.is_empty());
    println!("{}", swap_in(s, 4));
    println!("{z}");

    let mut w = 3i64;
    let one = std::array::from_mut(&mut w);
    one[0] += 4;
    println!("{} {w}", one.len());

    let mut o = Some(2u32);
    let s = slice::from_mut(&mut o);
    s[0] = s[0].map(|n| n * 10);
    println!("{o:?}");
    let mut none: Option<u32> = None;
    slice::from_mut(&mut none)[0] = Some(1);
    println!("{none:?}");

    let mut f = 1.5f64;
    double(&mut f);
    let mut n = 21i32;
    double(&mut n);
    println!("{f} {n}");

    let mut c = 'a';
    for item in slice::from_mut(&mut c) {
        *item = 'b';
    }
    println!("{c}");

    let mut flag = false;
    let view = slice::from_mut(&mut flag);
    view.fill(true);
    println!("{flag} {:?}", slice::from_ref(&flag));

    let mut k = 7u16;
    let (first, rest) = slice::from_mut(&mut k).split_first_mut().unwrap();
    *first += 1;
    println!("{} {}", rest.len(), k);
}
