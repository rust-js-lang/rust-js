// A range lent to `count()` or `last()`, `r.by_ref().count()`, is used up,
// as std's `fold` of a `&mut` leaves it: what's next of it after is
// nothing (#414).

fn main() {
    let mut a = 1u8..=3;
    let c = a.by_ref().count();
    println!("{c} {:?} {}", a.next(), a.is_empty());

    let mut b = 1u32..4;
    let d = b.by_ref().count();
    println!("{d} {:?} {:?}", b.next(), b);

    let mut e = 2i32..=6;
    let last = e.by_ref().last();
    println!("{last:?} {:?}", e.next());

    let mut f = 5usize..9;
    let last = (&mut f).last();
    println!("{last:?} {:?} {:?}", f.next(), f);

    let mut empty = 4u8..4;
    println!("{} {:?} {:?}", empty.by_ref().count(), empty.by_ref().last(), empty);

    let mut big = 1u64..=3;
    let n = big.by_ref().count();
    println!("{n} {:?}", big.next());

    // Not lent: what it's copied from is as it was.
    let g = 1u8..=3;
    println!("{} {:?}", g.clone().count(), g);
    let mut h = 1u8..=5;
    let taken = h.by_ref().take(2).count();
    println!("{taken} {:?}", h.next());
}
