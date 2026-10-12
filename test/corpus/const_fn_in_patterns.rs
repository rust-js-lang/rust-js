// A `const fn` behind a constant a pattern tests: rustc evaluates it as
// it builds the matching body's MIR, taking the `const fn`'s MIR, read
// then as rustc keeps it for evaluation.
const fn add(a: u32, b: u32) -> u32 {
    a + b
}

struct P {
    x: u32,
}

impl P {
    const fn new(x: u32) -> P {
        P { x }
    }
    const fn get(&self) -> u32 {
        self.x
    }
}

const SUM: u32 = add(1, 2);
const PX: u32 = P::new(7).get();

fn classify(n: u32) -> &'static str {
    match n {
        SUM => "sum",
        PX => "px",
        _ => "other",
    }
}

fn main() {
    println!("{} {} {} {} {}", classify(3), classify(7), classify(9), add(4, 5), P::new(2).get());
}
