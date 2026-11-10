use dep::{ptr_from_mut, same, slice_as_uninit_mut};
use std::mem::MaybeUninit;

pub fn main() {
    let mut n = 5u32;
    let p = ptr_from_mut(&mut n);
    unsafe { *p += 1 };
    println!("{n} {}", std::ptr::eq(p, &raw mut n));
    let mut bytes = [1u8, 2, 3];
    let uninit = slice_as_uninit_mut(&mut bytes);
    uninit[1] = MaybeUninit::new(20);
    println!("{bytes:?}");
    let mut words = vec![String::from("a")];
    let q = ptr_from_mut(&mut words);
    unsafe { (*q).push(String::from("b")) };
    let whole = same(&mut bytes[..] as *mut [u8]);
    unsafe { (*whole)[0] = 10 };
    println!("{words:?} {bytes:?}");
}
