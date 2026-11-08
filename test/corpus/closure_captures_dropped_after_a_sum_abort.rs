//@ run-abort
// A `map(f)` summed, whose `f` holds two values that each panic as they're
// dropped: dropped as the sum ends, the first's panic unwinds through the
// drop of the second, whose panic aborts there, as Rust's drop glue does.
// The drop the closure carries is an arrow made outside a function's body,
// which guards its cleanup where it's made (#938).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 >= 80 {
            panic!("drop {}", self.0);
        }
    }
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        let a = D(81);
        let b = D(82);
        let n: u32 = vec![1u32, 2]
            .into_iter()
            .map(move |x| {
                let _a = &a;
                let _b = &b;
                x + 1
            })
            .sum();
        println!("sum {n}");
    });
    println!("{}", r.is_err());
}
