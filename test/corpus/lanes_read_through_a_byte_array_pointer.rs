//@ library-differs
// ppv-lite86's generic vectors (#1402): a 16-byte slice read as a
// `[u8; 16]` through a raw pointer and transmuted to four `u32` lanes or two
// `u64` lanes, and back, little-endian as wasm32 is; and a slice of `u32`s
// read and written as a `[u32; 4]` through one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct U32x4([u32; 4]);

fn read_le(input: &[u8]) -> U32x4 {
    assert_eq!(input.len(), 16);
    let x: U32x4 = unsafe { core::mem::transmute(core::ptr::read(input as *const _ as *const [u8; 16])) };
    U32x4(x.0.map(u32::from_le))
}

fn read_be(input: &[u8]) -> U32x4 {
    assert_eq!(input.len(), 16);
    let x: U32x4 = unsafe { core::mem::transmute(core::ptr::read(input as *const _ as *const [u8; 16])) };
    U32x4(x.0.map(u32::from_be))
}

fn write_le(x: U32x4, out: &mut [u8]) {
    assert_eq!(out.len(), 16);
    let x = U32x4(x.0.map(u32::to_le));
    unsafe { core::ptr::write(out as *mut _ as *mut [u8; 16], core::mem::transmute(x)) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct U64x2([u64; 2]);

fn read_wide_le(input: &[u8]) -> U64x2 {
    assert_eq!(input.len(), 16);
    let x: U64x2 = unsafe { core::mem::transmute(core::ptr::read(input as *const _ as *const [u8; 16])) };
    U64x2(x.0.map(u64::from_le))
}

fn write_wide_be(x: U64x2, out: &mut [u8]) {
    let x = U64x2(x.0.map(u64::to_be));
    unsafe { core::ptr::write(out as *mut _ as *mut [u8; 16], core::mem::transmute(x)) }
}

fn first_four(words: &[u32]) -> [u32; 4] {
    unsafe { core::ptr::read(words as *const _ as *const [u32; 4]) }
}

fn set_first_four(words: &mut [u32], to: [u32; 4]) {
    unsafe { core::ptr::write(words as *mut _ as *mut [u32; 4], to) }
}

fn main() {
    let bytes: Vec<u8> = (1..=16).collect();
    let le = read_le(&bytes);
    println!("{le:x?}");
    println!("{:x?}", read_be(&bytes[..]));
    let mut out = [0u8; 20];
    write_le(U32x4([0x01020304, 0xa0b0c0d0, 7, u32::MAX]), &mut out[2..18]);
    println!("{out:?}");
    write_le(le, &mut out[4..]);
    println!("{out:?}");
    let wide = read_wide_le(&bytes);
    println!("{wide:x?}");
    write_wide_be(wide, &mut out[..16]);
    println!("{out:?}");
    let mut words = vec![1u32, 2, 3, 4, 5, 6];
    println!("{:?}", first_four(&words[1..]));
    set_first_four(&mut words[2..], [30, 40, 50, 60]);
    println!("{words:?}");
}
