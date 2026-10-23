// A range's own `nth`, `last` and `count`, of `u64`s too, are its
// bounds' arithmetic, as std's `Range`'s are, moving its start as `nth`
// does: of `a..=b` too, and of one reversed, whose `nth` is its `nth_back`
// and whose `skip` is its end moved down (#163). A range of `char`s steps
// by code point, its own way.
fn main() {
    println!("{:?} {:?}", (0..u64::MAX).nth(1 << 31), (0..u64::MAX).last());
    let mut r = 5..10;
    println!("{:?} {:?} {:?} {:?}", r.nth(2), r.next(), r.nth(5), r.next());
    let mut big = 10u64..12;
    println!("{:?} {:?} {:?}", big.nth(0), big.nth(1), big);
    println!("{:?} {:?} {} {}", (3..3).last(), (7..2).nth(0), (0..u32::MAX).count(), (4i32..-4).count());
    let mut neg = -5i64..0;
    println!("{:?} {:?}", neg.nth(4), neg.last());

    // Each finishes at once, where stepping each number wouldn't.
    println!("{:?}", (0..=u32::MAX).nth(4_000_000_000));
    println!("{:?}", (0..=u32::MAX).last());
    println!("{}", (0..=u16::MAX).count());
    println!("{:?}", (0..=u32::MAX).rev().nth(4_000_000_000));
    println!("{:?}", (7..1_000).rev().last());
    println!("{:?}", (0..u32::MAX).rev().skip(4_000_000_000).next());
    println!("{:?}", (0..u64::MAX).rev().nth(1 << 31));
    println!("{:?}", (0..=u64::MAX).rev().nth(1 << 31));

    // Past their ends, and empty.
    println!("{:?} {:?}", (1..=5).rev().nth(4), (1..=5).rev().nth(5));
    println!("{:?} {:?}", (5..=1).nth(0), (5..=1).rev().last());
    println!("{:?} {:?}", (3..3).rev().last(), (3..=3).rev().last());
    println!("{} {}", (5..=1).count(), (2..=2).count());

    // One held, stepped on after.
    let mut back = (10..20).rev();
    println!("{:?} {:?} {:?}", back.nth(2), back.next(), back.nth(100));
    println!("{:?}", back.next());
    let mut both = 1..=9;
    println!("{:?} {:?} {:?}", both.nth(3), both.next_back(), both.nth(3));
    println!("{:?} {:?}", both.next(), both.next_back());
    let mut down = (1..=9).rev();
    println!("{:?} {:?} {:?}", down.nth(3), down.next(), down.nth(10));

    // chars.
    println!("{:?} {:?}", ('a'..'z').nth(2), ('a'..'z').last());
    let mut letters = 'a'..'e';
    println!("{:?} {:?}", letters.nth(1), letters.next());
    println!("{:?} {:?}", ('a'..='e').rev().nth(1), ('a'..='e').count());
}
