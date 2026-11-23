//@ run-fail: attempt to divide by zero
// A division written to a place that's there already, `t.0 = t.1 / d`,
// panics where its division is, `t.1 / d`, not where its assignment
// starts: rustc writes the quotient straight to the place, so the MIR
// statement whose check the division's helper throws in place of spans
// the assignment (#2366, found by generated programs, seed 12467723).
fn divisor(d: u64) -> u64 {
    std::hint::black_box(d)
}

fn main() {
    let mut t = (0u64, 84u64);
    let mut q = 0u64;
    let r = &mut q;
    *r = 9 / divisor(3);
    t.0 = t.1 / divisor(2);
    println!("{} {}", t.0, q);
    t.0 = t.1 / divisor(0);
    println!("unreachable {t:?}");
}
