//@ native: wasm32
// Integer slices read as another width's, as rand_core 0.5.1 reads them
// (#1419): a `[u64]` slice's pointer as a `[u32]` slice's, its length kept,
// read and written through; two `u32`s read as a `u64`; integers copied into
// bytes and bytes into integers; one integer's bytes filled. Little-endian,
// wasm32's, where native Rust runs it.
use std::mem::size_of;
use std::ptr::{self, copy_nonoverlapping};
use std::slice;

// rand_core 0.5.1's block.rs:370: a `[u64]` slice read as `[u32]`, its
// length kept.
fn half(results: &[u64], index: usize) -> u32 {
    unsafe {
        let results = &*(results as *const [u64] as *const [u32]);
        *results.get_unchecked(index)
    }
}

// Written through such a view, the wider items change where they are.
fn set_halves(wide: &mut [u64]) {
    let narrow = unsafe { &mut *(wide as *mut [u64] as *mut [u32]) };
    narrow[0] = 7;
    narrow[1] = 1;
}

// block.rs:192: two `u32`s read as a `u64`.
fn read_u64(results: &[u32], index: usize) -> u64 {
    let ptr: *const u64 = results[index..=index + 1].as_ptr() as *const u64;
    unsafe { ptr::read_unaligned(ptr) }
}

// impls.rs's `fill_via_chunks!`: integers copied into bytes.
fn fill_from_u32(src: &[u32], dst: &mut [u8]) -> usize {
    let n = std::cmp::min(src.len() * 4, dst.len());
    unsafe { copy_nonoverlapping(src.as_ptr() as *const u8, dst.as_mut_ptr(), n) }
    n
}

fn fill_from_u64(src: &[u64], dst: &mut [u8]) -> usize {
    let n = std::cmp::min(src.len() * 8, dst.len());
    unsafe { copy_nonoverlapping(src.as_ptr() as *const u8, dst.as_mut_ptr(), n) }
    n
}

// le.rs's `read_slice!`: bytes copied into integers.
fn read_u32_into(src: &[u8], dst: &mut [u32]) {
    assert_eq!(src.len(), 4 * dst.len());
    unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr() as *mut u8, src.len()) }
    for v in dst.iter_mut() {
        *v = v.to_le();
    }
}

fn read_u64_into(src: &[u8], dst: &mut [u64]) {
    assert_eq!(src.len(), 8 * dst.len());
    unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr() as *mut u8, src.len()) }
    for v in dst.iter_mut() {
        *v = v.to_le();
    }
}

// impls.rs's `impl_uint_from_fill!`: one integer's bytes filled.
fn from_bytes(fill: impl Fn(&mut [u8])) -> u64 {
    let mut int: u64 = 0;
    unsafe {
        let ptr = &mut int as *mut u64 as *mut u8;
        let slice = slice::from_raw_parts_mut(ptr, size_of::<u64>());
        fill(slice);
    }
    int
}

fn main() {
    let wide = [0x0000_0002_0000_0001u64, 0x0000_0004_0000_0003, 0xdead_beef_cafe_f00d];
    println!("{} {} {} {}", half(&wide, 0), half(&wide, 1), half(&wide, 2), half(&wide[1..], 1));
    let mut halves = [9u64, 9];
    set_halves(&mut halves);
    println!("{halves:x?}");
    let narrow = [1u32, 2, 3, 0xffff_ffff];
    println!("{:#x} {:#x}", read_u64(&narrow, 0), read_u64(&narrow, 2));
    let mut bytes = [0u8; 10];
    println!("{} {:?}", fill_from_u32(&narrow, &mut bytes), bytes);
    let mut bytes = [0u8; 12];
    println!("{} {:?}", fill_from_u64(&wide, &mut bytes), bytes);
    let src: Vec<u8> = (1..=16).collect();
    let mut buf = [0u32; 4];
    read_u32_into(&src, &mut buf);
    println!("{:#x} {:#x}", buf[0], buf[3]);
    let mut buf = [0u32; 3];
    read_u32_into(&src[1..13], &mut buf);
    println!("{:#x} {:#x}", buf[0], buf[2]);
    let mut buf = [0u64; 2];
    read_u64_into(&src, &mut buf);
    println!("{:#x} {:#x}", buf[0], buf[1]);
    let n = from_bytes(|b| {
        for (i, x) in b.iter_mut().enumerate() {
            *x = i as u8 + 1;
        }
    });
    println!("{n:#x}");
}
