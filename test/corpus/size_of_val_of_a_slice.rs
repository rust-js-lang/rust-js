// `size_of_val` of a slice is its length's items' bytes, and `align_of_val`
// its item's alignment, as rand_core 0.10's `read_words` asserts of a
// `&[u8]` (#1101); of a slice of a wider item, a subslice's and an empty
// one's too. The slice measured is read once.
use std::mem::{align_of_val, size_of_val};

fn words(src: &[u8]) -> [u32; 2] {
    assert_eq!(size_of_val(src), size_of::<[u32; 2]>());
    let mut dst = [0u32; 2];
    for (out, chunk) in dst.iter_mut().zip(src.chunks_exact(4)) {
        *out = u32::from_le_bytes(chunk.try_into().unwrap());
    }
    dst
}

fn counted<'a>(reads: &mut u32, s: &'a [u64]) -> &'a [u64] {
    *reads += 1;
    s
}

fn main() {
    println!("{:?}", words(&[1, 0, 0, 0, 2, 1, 0, 0]));
    let wide = [1u64, 2, 3, 4, 5];
    let pairs = [(1u8, 2u16); 3];
    let mut reads = 0;
    println!(
        "{} {} {} {} {}",
        size_of_val(&wide[1..4]),
        size_of_val(&pairs[..]),
        size_of_val(&wide[..0]),
        align_of_val(&pairs[..]),
        size_of_val(counted(&mut reads, &wide[..]))
    );
    println!("{} {reads}", align_of_val(counted(&mut reads, &wide[2..])));
    let r = std::panic::catch_unwind(|| words(&[1, 2, 3]));
    println!("{}", r.is_err());
}
