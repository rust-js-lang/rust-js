//@ native: wasm32
// A mutable slice of another type's bytes, `from_raw_parts_mut` of a
// `*mut u32` cast to `*mut u8`, writes where its items are: each item read
// and written through the bytes wasm32 holds them as, little-endian, so
// what's written through it is what the array holds after (#1257); of
// `[u8; N]`s, each written where its bytes are (#1266). It was refused
// where it ran.
use std::slice;

#[repr(C, align(4))]
struct Aligned([u8; 8]);

fn bump(items: &mut [u16]) {
    for item in items.iter_mut() {
        *item = item.wrapping_add(0x0101);
    }
}

fn main() {
    // The issue's: words seen as their bytes, one written and all reversed.
    let mut a = [0x0102_0304u32, 5];
    let v = unsafe { slice::from_raw_parts_mut(a.as_mut_ptr().cast::<u8>(), 8) };
    v[1] = 0xff;
    v.reverse();
    println!("{a:?}");

    // Bytes seen as words, each written whole, by an iterator and sorted.
    let mut b = Aligned([1, 0, 0, 0, 2, 1, 0, 0]);
    let words = unsafe { slice::from_raw_parts_mut(b.0.as_mut_ptr().cast::<u32>(), 2) };
    words[0] = 0x0a0b_0c0d;
    for word in words.iter_mut() {
        *word += 1;
    }
    words.sort_unstable_by(|x, y| y.cmp(x));
    println!("{:?} {:?}", words, b.0);

    // Words seen as signed halves, given to a function, a part filled, a
    // part copied in, and swapped.
    let mut c = [0x8000_7fffu32, 0x0001_fffe];
    let halves = unsafe { slice::from_raw_parts_mut(c.as_mut_ptr().cast::<i16>(), 4) };
    println!("{halves:?}");
    halves[1] = -2;
    halves[2..].fill(-1);
    halves.swap(0, 3);
    println!("{halves:?} {c:x?}");
    let unsigned = unsafe { slice::from_raw_parts_mut(c.as_mut_ptr().cast::<u16>(), 4) };
    bump(&mut unsigned[1..3]);
    unsigned[..2].copy_from_slice(&[7, 8]);
    let (front, back) = unsigned.split_at_mut(2);
    front[1] = back[0];
    println!("{unsigned:?} {c:x?}");

    // Words seen as pairs of bytes: each pair written in place, whole, and
    // swapped, a pair read out of it kept as it was (#1266).
    let mut e = [0x0102_0304u32, 5];
    let pairs = unsafe { slice::from_raw_parts_mut(e.as_mut_ptr().cast::<[u8; 2]>(), 4) };
    pairs[0][1] = 9;
    let kept = pairs[1];
    pairs[1][0] = 3;
    pairs[2] = [6, 6];
    let last = &mut pairs[3];
    last[0] = 1;
    pairs.swap(0, 2);
    println!("{kept:?} {pairs:?} {e:?}");

    // A pointer into it is into the array's bytes: written through, and at
    // the address the array's bytes are.
    let mut d = [0u32; 2];
    let bytes = unsafe { slice::from_raw_parts_mut(d.as_mut_ptr().cast::<u8>(), 8) };
    let p = bytes[2..].as_mut_ptr();
    unsafe {
        *p.add(1) = 9;
        p.write(4);
    }
    let offset = p as usize - d.as_ptr() as usize;
    println!("{d:?} {offset}");
}
