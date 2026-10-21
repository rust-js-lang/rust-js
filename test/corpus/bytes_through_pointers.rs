// Bytes read and written through a pointer (ADR 0562): a byte slice read as
// wider integers at any offset, as aho-corasick's `is_equal_raw` and
// memchr's word loads do, a wider array read as bytes, and plain data's
// bytes as wasm32 lays them out, little-endian.
use std::mem::MaybeUninit;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pair {
    low: u16,
    high: u16,
    tag: u8,
}

// aho-corasick's byte-at-a-time comparison, four at a time.
unsafe fn is_equal_raw(mut x: *const u8, mut y: *const u8, n: usize) -> bool {
    if n < 4 {
        return match n {
            0 => true,
            1 => unsafe { x.read() == y.read() },
            2 => unsafe { x.cast::<u16>().read_unaligned() == y.cast::<u16>().read_unaligned() },
            3 => unsafe { x.cast::<[u8; 3]>().read() == y.cast::<[u8; 3]>().read() },
            _ => unreachable!(),
        };
    }
    let xend = unsafe { x.add(n.wrapping_sub(4)) };
    let yend = unsafe { y.add(n.wrapping_sub(4)) };
    while x < xend {
        let vx = unsafe { x.cast::<u32>().read_unaligned() };
        let vy = unsafe { y.cast::<u32>().read_unaligned() };
        if vx != vy {
            return false;
        }
        x = unsafe { x.add(4) };
        y = unsafe { y.add(4) };
    }
    unsafe { xend.cast::<u32>().read_unaligned() == yend.cast::<u32>().read_unaligned() }
}

fn main() {
    let bytes: Vec<u8> = (1..=12).collect();
    let p = bytes.as_ptr();
    let words: Vec<u32> = (0..3)
        .map(|i| unsafe { p.add(i * 4).cast::<u32>().read_unaligned() })
        .collect();
    let unaligned = unsafe { p.add(1).cast::<u32>().read_unaligned() };
    let half = unsafe { p.add(5).cast::<u16>().read_unaligned() };
    let three = unsafe { p.add(9).cast::<[u8; 3]>().read() };
    let signed = unsafe { p.add(11).cast::<i8>().read() };
    let stepped = unsafe { p.cast::<u32>().add(2).read_unaligned() };
    println!("{words:?} {unaligned:#x} {half:#x} {three:?} {signed} {stepped}");

    for (a, b, n) in [
        (b"abcdefgh".as_slice(), b"abcdefgh".as_slice(), 8),
        (b"abcdefgh", b"abcdefgx", 8),
        (b"abc", b"abc", 3),
        (b"ab", b"ac", 2),
        (b"abcde", b"abcdf", 5),
    ] {
        print!("{} ", unsafe { is_equal_raw(a.as_ptr(), b.as_ptr(), n) });
    }
    println!();

    // Written through as a wider integer, then read as bytes again.
    let mut buffer = [0u8; 8];
    let b = buffer.as_mut_ptr();
    unsafe {
        b.add(1).cast::<u32>().write_unaligned(0xdead_beef);
        b.add(6).cast::<u16>().write(0x4142);
        *b.add(5) = 7;
    }
    let negative = unsafe { b.add(1).cast::<i8>().read() };
    let text: Box<str> = unsafe { std::mem::transmute::<Box<[u8]>, Box<str>>(b"bytes".to_vec().into_boxed_slice()) };
    println!("{buffer:?} {negative} {text}");

    // A wider array's bytes.
    let mut wide = [0x0403_0201u32, 0x0807_0605];
    let w = wide.as_mut_ptr().cast::<u8>();
    let tail = unsafe { w.add(3).cast::<u32>().read_unaligned() };
    let copied: Vec<u8> = unsafe { std::slice::from_raw_parts(w.add(2), 5) }.to_vec();
    unsafe {
        *w.add(1) = 0xff;
        ptr::copy_nonoverlapping(bytes.as_ptr(), w.add(4), 3);
    }
    println!("{tail:#x} {copied:?} {:#x} {:#x}", wide[0], wide[1]);

    // Plain data's bytes, read and written through a pointer to them.
    let mut pair = Pair {
        low: 0x1122,
        high: 0x3344,
        tag: 9,
    };
    let q = &raw mut pair as *mut u8;
    let first = unsafe { q.read() };
    let high = unsafe { q.add(2).cast::<u16>().read() };
    let all = unsafe { std::slice::from_raw_parts(q, 5) }.to_vec();
    unsafe {
        *q.add(4) = 10;
        q.add(1).cast::<u16>().write_unaligned(0xaabb);
    }
    println!("{first:#x} {high:#x} {all:?} {pair:?}");

    // An uninitialized slot's bytes, written one by one, then read whole.
    let mut slot = MaybeUninit::<u32>::uninit();
    let s = (&raw mut slot).cast::<u8>();
    for i in 0..4 {
        unsafe { s.add(i).write(0x10 * (i as u8 + 1)) };
    }
    println!("{:#x}", unsafe { slot.assume_init() });
    let mut raw = [MaybeUninit::<u8>::uninit(); 4];
    let r = raw.as_mut_ptr();
    for i in 0..4 {
        unsafe { r.add(i).write(MaybeUninit::new(i as u8 + 1)) };
    }
    println!("{:#x}", unsafe { r.cast::<u32>().read_unaligned() });
    println!(
        "{}",
        unsafe { ptr::read_unaligned(&raw const pair as *const u32) } == u32::from_le_bytes([0xbb, 0xaa, 0x44, 0x33])
    );
}
