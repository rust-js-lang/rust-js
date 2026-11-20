// `Ord`'s `max`, `min` and `cmp` of `&mut` references to numbers and text:
// std's `Ord for &mut A`, which compares what they refer to and gives the
// chosen reference, `max`'s the second of two equal ones, never JS's
// `Math.max` of their handles (#2188). A shared reference's are the
// number's, JS's own.
use std::cmp::Ordering;

fn main() {
    let (mut a, mut b) = (1, 2);
    let (ra, rb) = (&mut a, &mut b);
    println!("{}", ra.max(rb));

    let (mut c, mut d) = (5, 3);
    let (rc, rd) = (&mut c, &mut d);
    *rc.min(rd) += 10;
    println!("{c} {d}");

    let (mut e, mut f) = (4, 4);
    *(&mut e).max(&mut f) += 1;
    println!("{e} {f}");

    let (mut g, mut h) = (7, 9);
    let (rg, rh) = (&mut g, &mut h);
    let ordering = rg.cmp(&rh);
    println!("{}", ordering == Ordering::Less);

    let (mut u, mut v) = (String::from("x"), String::from("w"));
    (&mut u).min(&mut v).push('!');
    println!("{u} {v}");

    let (mut i, mut j) = (1, 2);
    *std::cmp::max(&mut i, &mut j) += 10;
    *std::cmp::min(&mut i, &mut j) += 100;
    println!("{i} {j}");

    let (p, q) = (3, 8);
    println!("{} {}", (&p).max(&q), (&p).min(&q));
}
