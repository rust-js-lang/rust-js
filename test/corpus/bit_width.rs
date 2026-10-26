// `bit_width()` of an unsigned integer, its bits but its leading zeros, of
// each width: `32 - Math.clz32(n)` of one of 32 bits or fewer, a 64- or
// 128-bit one's of its own; as binary lifting Fibonacci steps through `n`'s
// bits from its highest.
fn fibonacci(n: u32) -> u128 {
    let mut state = (0u128, 1u128);
    for i in (0..n.bit_width()).rev() {
        state = (state.0 * (2 * state.1 - state.0), state.0 * state.0 + state.1 * state.1);
        if n & (1 << i) != 0 {
            state = (state.1, state.0 + state.1);
        }
    }
    state.0
}

fn main() {
    let small = [0u8, 1, 7, 128, 255];
    let words = [0u32, 1, 0b1110, u32::MAX];
    let wide = [0u64, 1 << 40, u64::MAX];
    let huge = [0u128, 1 << 100, u128::MAX];
    let sizes = [0usize, 1000];
    println!(
        "{:?} {:?} {:?} {:?} {:?}",
        small.map(|n| n.bit_width()),
        words.map(|n| n.bit_width()),
        wide.map(|n| n.bit_width()),
        huge.map(|n| n.bit_width()),
        sizes.map(|n| n.bit_width()),
    );
    println!("{:?}", [0, 1, 2, 10, 50, 100].map(fibonacci));
}
