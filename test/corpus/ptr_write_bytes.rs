// `ptr::write_bytes(dst, byte, count)` and `dst.write_bytes(byte, count)`:
// each of the `count` items from where `dst` points made of bytes `byte`,
// as getrandom's `uninit_slice_fill_zero` zeroes a `MaybeUninit<u8>`
// buffer, and of wider integers, floats and plain structs, their bytes as
// wasm32 lays them out (#1133), each read back as its type is (#1137).
use std::mem::MaybeUninit;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pair {
    low: u16,
    high: u32,
}

// getrandom 0.3's own.
fn uninit_slice_fill_zero(slice: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    unsafe { ptr::write_bytes(slice.as_mut_ptr(), 0, slice.len()) };
    unsafe { &mut *(slice as *mut [MaybeUninit<u8>] as *mut [u8]) }
}

fn main() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 4];
    println!("{:?}", uninit_slice_fill_zero(&mut buffer));

    let mut bytes = [1u8, 2, 3, 4, 5];
    unsafe { bytes.as_mut_ptr().add(1).write_bytes(0xAB, 3) };
    println!("{bytes:?}");

    let mut words = [7u32; 3];
    unsafe { ptr::write_bytes(words.as_mut_ptr().add(1), 0x01, 2) };
    println!("{words:?}");

    let mut halves = [0i16; 2];
    unsafe { ptr::write_bytes(halves.as_mut_ptr(), 0xFF, 1) };
    println!("{halves:?}");

    let mut wide = [0u64; 2];
    unsafe { ptr::write_bytes(wide.as_mut_ptr(), 0x80, 2) };
    println!("{wide:?}");

    // Through a pointer to bytes of a wider array: three of its bytes.
    let mut big = [0u32; 2];
    unsafe { ptr::write_bytes(big.as_mut_ptr().cast::<u8>().add(1), 0xFF, 3) };
    println!("{big:x?}");

    let mut pair = Pair { low: 1, high: 2 };
    unsafe { ptr::write_bytes(&mut pair as *mut Pair, 0, 1) };
    println!("{pair:?}");

    // A byte written into a signed integer's, read back as its type is: -1
    // of an `i16` whose bytes are all `0xFF` (#1137), and a float's.
    let mut signed = [0i16; 2];
    let p = signed.as_mut_ptr().cast::<u8>();
    unsafe {
        *p = 0xFF;
        *p.add(1) = 0xFF;
    }
    println!("{signed:?}");
    let mut longs = [0i64; 1];
    unsafe { ptr::write_bytes(longs.as_mut_ptr(), 0xFE, 1) };
    println!("{longs:?}");
    let mut floats = [0f32; 1];
    unsafe { ptr::write_bytes(floats.as_mut_ptr(), 0x3F, 1) };
    println!("{floats:?}");
    let mut pairs = [Pair { low: 1, high: 2 }; 2];
    unsafe { ptr::write_bytes(pairs.as_mut_ptr().add(1), 0x01, 1) };
    println!("{pairs:?}");

    let byte: u8 = std::hint::black_box(0x11);
    let mut none = [9u8; 2];
    unsafe { ptr::write_bytes(none.as_mut_ptr(), byte, 0) };
    unsafe { ptr::write_bytes(none.as_mut_ptr(), byte, 1) };
    println!("{none:?}");
}
