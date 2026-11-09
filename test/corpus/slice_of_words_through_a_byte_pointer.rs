// A slice made of a byte pointer cast to a wider integer's, `*const u32`,
// is of its words, each read from its 4 bytes, little-endian, as wasm32
// holds them; an `*const i8`'s of signed bytes, and a `*const [u8; 2]`'s
// of pairs (#1107). It was each byte, the words' first ones.
#[repr(C, align(4))]
struct Aligned([u8; 8]);

fn main() {
    let a = Aligned([1, 0, 0, 0, 2, 1, 0, 0]);
    let words = unsafe { std::slice::from_raw_parts(a.0.as_ptr() as *const u32, 2) };
    println!("{words:?}");
    let b = Aligned([0, 0, 0, 0, 0xff, 0x80, 7, 0]);
    let signed = unsafe { std::slice::from_raw_parts((b.0.as_ptr() as *const i8).add(4), 3) };
    println!("{signed:?}");
    let pairs = unsafe { std::slice::from_raw_parts(a.0.as_ptr() as *const [u8; 2], 4) };
    println!("{pairs:?}");
}
