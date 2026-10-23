// A range's own `nth`, `last` and `count`, of `u64`s too, are its
// bounds' arithmetic, as std's `Range`'s are, moving its start as `nth`
// does.
fn main() {
    println!("{:?} {:?}", (0..u64::MAX).nth(1 << 31), (0..u64::MAX).last());
    let mut r = 5..10;
    println!("{:?} {:?} {:?} {:?}", r.nth(2), r.next(), r.nth(5), r.next());
    let mut big = 10u64..12;
    println!("{:?} {:?} {:?}", big.nth(0), big.nth(1), big);
    println!("{:?} {:?} {} {}", (3..3).last(), (7..2).nth(0), (0..u32::MAX).count(), (4i32..-4).count());
    let mut neg = -5i64..0;
    println!("{:?} {:?}", neg.nth(4), neg.last());
}
