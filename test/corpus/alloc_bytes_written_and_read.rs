// An allocation of `alloc`'s used as bytes, as semver's heap identifiers
// are (#1099): a length's varint written byte by byte through the
// `*mut u8`, then a string's bytes copied after it, read back as a slice,
// a `u16` read unaligned, a byte rewritten, and a `Vec<u8>` made of a
// second one's bytes, which owns them. Each slot of the allocation is a
// byte, as wasm32's are.
use std::alloc::{Layout, alloc, dealloc};
use std::ptr;

fn bytes_for_varint(len: usize) -> usize {
    let bits = usize::BITS - len.leading_zeros();
    bits.div_ceil(7) as usize
}

fn make(text: &str) -> (*mut u8, Layout) {
    let len = text.len();
    let layout = Layout::from_size_align(bytes_for_varint(len) + len, 2).unwrap();
    let ptr = unsafe { alloc(layout) };
    let mut write = ptr;
    let mut remaining = len;
    while remaining > 0 {
        unsafe { ptr::write(write, remaining as u8 | 0x80) };
        remaining >>= 7;
        write = unsafe { write.add(1) };
    }
    unsafe { ptr::copy_nonoverlapping(text.as_ptr(), write, len) };
    (ptr, layout)
}

fn read<'a>(ptr: *const u8, len: usize) -> &'a str {
    let at = unsafe { ptr.add(bytes_for_varint(len)) };
    let bytes = unsafe { std::slice::from_raw_parts(at, len) };
    std::str::from_utf8(bytes).unwrap()
}

fn main() {
    let text = "alpha.beta.gamma.delta.epsilon";
    let (p, layout) = make(text);
    println!("{} {}", unsafe { *p }, read(p, text.len()));
    let pair = unsafe { (p.add(1) as *const u16).read_unaligned() };
    println!("{pair:#06x}");
    unsafe { *p.add(1) = b'A' };
    println!("{}", read(p, text.len()));
    unsafe { dealloc(p, layout) };

    let layout = Layout::from_size_align(4, 1).unwrap();
    let q = unsafe { alloc(layout) };
    for i in 0..4 {
        unsafe { q.add(i).write(b'w' + i as u8) };
    }
    let owned = unsafe { Vec::from_raw_parts(q, 4, 4) };
    println!("{:?} {}", owned, String::from_utf8(owned.clone()).unwrap());
}
