// A range's `map(f)` folded from its back, `rfold`, by a closure that holds
// what drops: the range a map steps from its back made knowing its back
// where the map is made, `as_double_ended`, as its MIR holds it in a
// temporary across the closure's capture, so the map's `rfold` steps it
// from its end (#863, which found that one place does this of it, where
// two did).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn main() {
    let mut seen = 0;
    let g = D(90);
    let counter = &mut seen;
    let acc = (1..6u32).map(|x| x + 100).rfold(Acc(0), move |mut a, x| {
        let _g = &g;
        *counter += 1;
        a.0 = a.0 * 1000 % 1_000_003 + x;
        a
    });
    println!("rfold {} after {seen}", acc.0);
}
