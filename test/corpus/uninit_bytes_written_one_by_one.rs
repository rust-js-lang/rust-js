//@ native: wasm32
// A `MaybeUninit` of a number written a byte at a time through a view of
// its bytes, as getrandom's `inner_u64` fills one, is those bytes (#1279).
// Its value isn't there till it's written: the view starts from bytes, not
// from encoding what isn't there, which for a 64-bit or 128-bit number
// threw "Cannot convert undefined to a BigInt". Each width and sign, the
// floats, and an array of them. A mutable slice of one place, a `u8`'s,
// is a view of it, written where it is (#1280); it was a copy.
use std::mem::MaybeUninit;
use std::slice;

macro_rules! filled {
    ($t:ty) => {{
        let mut res = MaybeUninit::<$t>::uninit();
        let n = size_of::<$t>();
        let dst = unsafe { slice::from_raw_parts_mut(res.as_mut_ptr().cast::<MaybeUninit<u8>>(), n) };
        for (i, byte) in dst.iter_mut().enumerate() {
            byte.write(0x11 * (i as u8 % 3 + 1));
        }
        unsafe { res.assume_init() }
    }};
}

fn main() {
    println!("{:#x} {:#x} {:#x} {:#x} {:#x}", filled!(u8), filled!(u16), filled!(u32), filled!(u64), filled!(u128));
    println!("{} {} {} {} {}", filled!(i8), filled!(i16), filled!(i32), filled!(i64), filled!(i128));
    println!("{} {}", filled!(f32), filled!(f64));

    // One byte written through a pointer, the rest after.
    let mut one = MaybeUninit::<u64>::uninit();
    let p = one.as_mut_ptr().cast::<u8>();
    unsafe {
        p.add(7).write(0x80);
        for i in 0..7 {
            p.add(i).write(i as u8);
        }
    }
    println!("{:#x}", unsafe { one.assume_init() });

    // An array of them, each item's bytes written.
    let mut items = [MaybeUninit::<u64>::uninit(); 2];
    let bytes = unsafe { slice::from_raw_parts_mut(items.as_mut_ptr().cast::<u8>(), 16) };
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = i as u8;
    }
    println!("{:#x} {:#x}", unsafe { items[0].assume_init() }, unsafe { items[1].assume_init() });

    // A slice of one place, written through, and where the place is.
    let mut x = 5u8;
    let s = unsafe { slice::from_raw_parts_mut(&raw mut x, 1) };
    s[0] = 9;
    let at = std::ptr::eq(s.as_ptr(), &raw const x);
    println!("{x} {at}");
}
