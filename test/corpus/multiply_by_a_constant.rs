// A 32-bit multiply by a constant of at most 2^21 either side is JS's own
// `*`, wrapped as `+`'s is, `(n * 2) | 0`: its product is under 2^53, exact
// in a double (ADR 0011). Past that, or of two variables, `Math.imul`.
// Each wraps as release Rust does, at the edges of its type.
const SCALE: i32 = 3;

fn double(n: i32) -> i32 {
    n.wrapping_mul(2)
}

fn main() {
    let edges = [i32::MAX, i32::MIN, -1, 0, 123_456_789];
    for n in edges {
        println!(
            "{} {} {} {}",
            double(n),
            n.wrapping_mul(-2_097_152),
            (-7i32).wrapping_mul(n),
            n.wrapping_mul(2_097_153)
        );
    }
    // A named constant's value, which rustc knows, through `*`.
    let small = [1000, -7];
    println!("{} {}", small[0] * SCALE, small[1] * SCALE);
    let unsigned = [u32::MAX, 1 << 31, 7];
    for n in unsigned {
        // Past 2^21, a product past 2^53, rounded in a double: `Math.imul`.
        println!(
            "{} {} {} {}",
            n.wrapping_mul(3),
            n.wrapping_mul(2_097_152),
            n.wrapping_mul(4_194_305),
            n.wrapping_mul(n)
        );
    }
}
