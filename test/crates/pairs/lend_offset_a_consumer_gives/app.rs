//! A consumer of the library's generic lends, giving each its instances,
//! whose payloads are at different offsets: a `Result<u8, u16>`'s `Ok` at 1
//! and a `Result<u16, u8>`'s at 2, an `Err` of each at 2 and 1. Each
//! pointer at its own, and a write through it the payload's (#1478).
pub fn main() {
    let mut a: Result<u8, u16> = Ok(1);
    let mut b: Result<u16, u8> = Ok(2);
    let (base_a, base_b) = (&raw const a as usize, &raw const b as usize);
    let (pa, pb) = (dep::ok_ptr(&mut a), dep::ok_ptr(&mut b));
    unsafe {
        *pa += 10;
        *pb += 20;
    }
    println!("ok {} {} {a:?} {b:?}", pa as usize - base_a, pb as usize - base_b);
    let mut c: Result<u8, u16> = Err(3);
    let mut d: Result<u16, u8> = Err(4);
    let (base_c, base_d) = (&raw const c as usize, &raw const d as usize);
    let (pc, pd) = (dep::err_ptr(&mut c), dep::err_ptr(&mut d));
    unsafe {
        *pc += 1;
        *pd += 1;
    }
    println!("err {} {} {c:?} {d:?}", pc as usize - base_c, pd as usize - base_d);
    println!("{}", dep::address_of(&a) != 0);
}
