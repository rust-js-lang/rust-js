//! A consumer naming std's items through its library's re-exports, which
//! are std's own (rd802, #1315).
pub fn main() {
    let (mut a, mut b) = (1u8, 2u8);
    dep::named::swap(&mut a, &mut b);
    let swap: fn(&mut u8, &mut u8) = dep::core_reexport::mem::swap;
    swap(&mut a, &mut b);
    swap(&mut a, &mut b);
    let mut v = vec![dep::twice(3)];
    let old = dep::core_reexport::mem::replace(&mut v, vec![9]);
    println!("{a} {b} {old:?} {v:?} {}", dep::core_reexport::mem::size_of::<u16>());
}
