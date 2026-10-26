// A `Wrapping`'s `op=` through a `&mut`: a parameter's, an iterator's, and
// the borrow MIR takes before an operand that's a call, `b0 += b1 << 4`,
// each written to what it points at, as TEA's rounds are.
use std::num::Wrapping as W;

fn bump(r: &mut W<u32>) {
    *r += W(1);
}

fn halves(n: u64) -> (W<u32>, W<u32>) {
    (W(n as u32), W((n >> 32) as u32))
}

fn mix(block: u64) -> u32 {
    let (mut b0, b1) = halves(block);
    for _ in 0..3 {
        b0 += b1 << 4;
        b0 -= (b0 >> 3) ^ b1;
    }
    b0.0
}

fn main() {
    let mut a = W(u32::MAX);
    bump(&mut a);
    let mut bytes = vec![W(1u8), W(255)];
    for w in bytes.iter_mut() {
        *w += W(1);
    }
    println!("{a} {bytes:?} {}", mix(0x1_0000_0002));
}
