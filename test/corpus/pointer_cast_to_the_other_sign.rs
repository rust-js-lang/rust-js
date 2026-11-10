//@ native: wasm32
// An integer array read and written through a pointer to the same width's
// other sign, or to bytes: each item's bytes as wasm32 holds them, an
// `i8`'s `-21` the byte `235`, through its codec, as any other integer's
// (#1253, ADR 0562); found by the memory net (#1172).
use std::{ptr, slice};

fn main() {
    // An `i8` array's bytes, read, viewed and written as `u8`s.
    let mut signed: [i8; 4] = [-21, 5, i8::MIN, -1];
    let bytes = unsafe { slice::from_raw_parts(signed.as_ptr().cast::<u8>(), 4) };
    println!("{bytes:?}");
    let first = unsafe { signed.as_ptr().cast::<u8>().read() };
    let last = unsafe { *(signed.as_ptr().add(3) as *const u8) };
    println!("{first} {last}");
    unsafe { signed.as_mut_ptr().cast::<u8>().write(235) };
    unsafe { *signed.as_mut_ptr().cast::<u8>().add(1) = 200 };
    println!("{signed:?}");
    let pair = unsafe { signed.as_ptr().cast::<u8>().add(1).cast::<u16>().read_unaligned() };
    println!("{pair:#x}");

    // A `u8` array's bytes as `i8`s, and written as them.
    let mut unsigned: [u8; 4] = [235, 5, 128, 255];
    let viewed = unsafe { slice::from_raw_parts(unsigned.as_ptr().cast::<i8>(), 4) };
    println!("{viewed:?}");
    unsafe { unsigned.as_mut_ptr().cast::<i8>().add(2).write(-2) };
    println!("{unsigned:?}");

    // Wider integers' bytes as `i8`s, and the same width's other sign.
    let words: [u32; 2] = [0x80ff_0102, 7];
    println!("{:?}", unsafe { slice::from_raw_parts(words.as_ptr().cast::<i8>(), 8) });
    let halves: [i16; 2] = [-2, i16::MIN];
    println!("{:?}", unsafe { slice::from_raw_parts(halves.as_ptr().cast::<u16>(), 2) });
    let mut longs: [u64; 2] = [u64::MAX, 1 << 63];
    println!("{:?}", unsafe { slice::from_raw_parts(longs.as_ptr().cast::<i64>(), 2) });
    unsafe { longs.as_mut_ptr().cast::<i64>().write(-5) };
    println!("{longs:?}");

    // Bytes copied, and filled, across the sign.
    let mut into: [i8; 4] = [0; 4];
    unsafe { ptr::copy_nonoverlapping(unsigned.as_ptr(), into.as_mut_ptr().cast::<u8>(), 4) };
    println!("{into:?}");
    let mut back: [u8; 4] = [0; 4];
    unsafe { ptr::copy_nonoverlapping(signed.as_ptr().cast::<u8>(), back.as_mut_ptr(), 4) };
    println!("{back:?}");
    unsafe { ptr::write_bytes(into.as_mut_ptr().add(1), 0xff, 2) };
    println!("{into:?}");
    let as_bytes = unsafe { signed.as_ptr().cast::<[u8; 2]>().read() };
    println!("{as_bytes:?}");
}
