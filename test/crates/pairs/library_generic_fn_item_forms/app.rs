use dep::{apply_ptr, measured};

pub fn main() {
    let mut bytes = [1u8, 2, 3];
    let q = apply_ptr(&mut bytes[..] as *mut [u8], measured::<[u8]>);
    unsafe { (*q)[0] = 9 };
    let r = apply_ptr(&mut bytes[..] as *mut [u8], |p| measured::<[u8]>(p));
    unsafe { (*r)[1] = 8 };
    println!("{bytes:?}");
}
