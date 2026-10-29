// A `RangeInclusive` stepped past its type's last integer, `127i8..=127`,
// or before its first: release Rust wraps that bound and marks it
// exhausted, which its `start()`, `end()`, `{:?}`, `==`, `contains`,
// `is_empty()` and bounds read (#414).

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

fn show(r: &RangeInclusive<i8>) {
    println!(
        "{:?} {:?} {:?} {} {} {}",
        r,
        r.start(),
        r.end(),
        r.is_empty(),
        r.contains(&0),
        r == &(-128..=127)
    );
}

fn main() {
    let mut r = 127i8..=127;
    println!("{:?}", r.next());
    show(&r);
    println!("{:?} {:?}", r.next(), r.clone().into_inner());

    let mut back = -128i8..=-128;
    println!("{:?}", back.next_back());
    show(&back);

    let mut u = 250u8..=255;
    println!("{:?} {:?}", u.nth(10), u);
    let mut v = 1u8..=5;
    println!("{:?} {:?} {}", v.nth(9), v, v.is_empty());
    let mut w = 1u8..=5;
    println!("{:?} {:?}", w.nth_back(1), w);

    let mut wide = u32::MAX - 1..=u32::MAX;
    println!("{:?} {:?} {:?}", wide.next(), wide.next(), wide);
    let mut big = i64::MAX..=i64::MAX;
    println!("{:?} {:?} {:?}", big.next(), big, big.start());

    let map: BTreeMap<i8, &str> = [(-128, "min"), (0, "zero"), (127, "max")].into_iter().collect();
    println!("{:?}", map.range(r.clone()).collect::<Vec<_>>());
    let fresh = -128i8..=127;
    println!("{:?} {}", map.range(fresh.clone()).count(), fresh == r);

    // Used up by a fold, through `by_ref()`: exhausted at its last, as
    // std's `try_fold` leaves it, its clone too.
    let mut f = 1u8..=3;
    let all: u32 = f.by_ref().map(u32::from).sum();
    let copy = f.clone();
    println!("{all} {f:?} {copy:?} {} {}", f.is_empty(), f == copy);
    let mut g = 1u8..=4;
    println!("{} {g:?}", g.by_ref().any(|x| x == 4));
    let mut h = 1u8..=4;
    println!("{} {h:?}", h.by_ref().any(|x| x == 2));
    let mut k = 1u8..=3;
    println!("{} {k:?} {:?}", k.by_ref().rev().fold(0, |a, x| a * 10 + x), k.start());
    let mut c = 'x'..='z';
    println!("{} {c:?}", c.by_ref().fold(String::new(), |s, x| s + &x.to_string()));
    let mut last = char::MAX..=char::MAX;
    println!("{:?} {last:?}", last.next());

    // Collected through `by_ref()`: as std's `SpecFromIter` takes it, by its
    // `fold` where it's `TrustedLen`, by `next` where it's not, a filter's,
    // whose `next` is a `find`, a `try_fold`; a string's always by `fold`.
    let mut a = 1u8..=3;
    let v: Vec<u8> = a.by_ref().collect();
    println!("{v:?} {a:?}");
    let mut b = 1u8..=3;
    let w: Vec<u8> = b.by_ref().map(|x| x * 2).collect();
    println!("{w:?} {b:?}");
    let mut c = 1u8..=3;
    let odd: Vec<u8> = c.by_ref().filter(|x| x % 2 == 1).collect();
    println!("{odd:?} {c:?}");
    let mut d = 'x'..='z';
    println!("{} {d:?}", d.by_ref().filter(|x| *x != 'y').count());
    let mut e = 1u8..=3;
    for x in e.by_ref().map(|x| x + 1) {
        print!("{x} ");
    }
    println!("{e:?}");
    let mut t = 1u8..=3;
    let text: String = t.by_ref().map(|x| x.to_string()).collect();
    println!("{text} {t:?}");
    let mut k = 1u8..=4;
    let two: Vec<u8> = k.by_ref().take(2).collect();
    println!("{two:?} {k:?}");

    // Stepped to its end below its type's last: no wrap, not exhausted.
    let mut s = 1u8..=3;
    while s.next().is_some() {}
    println!("{:?} {} {:?}", s, s.is_empty(), s.start());
}
