// An array a raw pointer is taken into, given by `&mut` to `mem::replace`,
// `mem::swap` or `mem::take`, either side of a swap: its items written
// over in place, as an object given by `&mut` is, so the pointer reads
// the new item where it was, as Rust's does (#1616). Where the variable was
// given its new value, the pointer read the old array's item.
#[derive(Debug, Default)]
struct N(u8, u32);

fn replace_through(slot: &mut [N; 2]) -> [N; 2] {
    std::mem::replace(slot, [N(9, 9), N(8, 8)])
}

fn main() {
    let mut whole = [N(1, 2), N(3, 4)];
    let q = &raw const whole[0];
    let old = std::mem::replace(&mut whole, [N(5, 6), N(7, 8)]);
    println!("{old:?} {whole:?} {:?}", unsafe { &*q });
    let older = replace_through(&mut whole);
    println!("{older:?} {whole:?} {:?}", unsafe { &*q });
    let mut a = [N(1, 2), N(3, 4)];
    let mut b = [N(5, 6), N(7, 8)];
    let pa = &raw const a[0];
    std::mem::swap(&mut a, &mut b);
    println!("{a:?} {b:?} {:?}", unsafe { &*pa });
    let pb = &raw const b[1];
    let taken = std::mem::take(&mut b);
    println!("{taken:?} {b:?} {:?}", unsafe { &*pb });
    let mut c = [N(9, 9), N(8, 8)];
    let pc = &raw const c[0];
    let mut d = [N(0, 0), N(1, 1)];
    std::mem::swap(&mut d, &mut c);
    println!("{c:?} {d:?} {:?}", unsafe { &*pc });
}
