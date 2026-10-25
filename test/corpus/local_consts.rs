// A `const` written in a function's body is a `const` there, as React's
// ReactFiberLane writes `includesBlockingLane`'s `SyncDefaultLanes`, and
// another function's of the same name is too, apart (ADR 0031, amended).
// Where something before it reads it, as Rust's may, it's before that.
// One a function of the module's, or another constant, reads is the
// module's.

const SYNC: u32 = 1;
const INPUT: u32 = 4;
const BASE: u32 = 7;

fn includes_blocking_lane(lanes: u32) -> bool {
    // Every lane a render blocks on.
    #[cfg_attr(rust_js, rust_js::name = "SyncDefaultLanes")]
    const SYNC_DEFAULT_LANES: u32 = SYNC | INPUT | 16;
    (lanes & SYNC_DEFAULT_LANES) != 0
}

fn is_blocking_lane(lane: u32) -> bool {
    #[cfg_attr(rust_js, rust_js::name = "SyncDefaultLanes")]
    const SYNC_DEFAULT_LANES: u32 = SYNC | INPUT | 16;
    (lane & SYNC_DEFAULT_LANES) != 0
}

// Read before it's written, and in a closure and a function of its own.
fn clamped(values: &[u32]) -> Vec<u32> {
    let limit = |x: u32| x.min(LIMIT);
    let out = values.iter().map(|&x| step(limit(x))).collect();
    const LIMIT: u32 = 40 + 2;
    fn step(x: u32) -> u32 {
        x + LIMIT
    }
    out
}

// Read before it's written, and by a closure.
fn capped(values: &[u32]) -> u32 {
    let floor = values.len() as u32 + CAP;
    const CAP: u32 = 7;
    let cap = |x: u32| x.min(CAP);
    values.iter().map(|&x| cap(x)).sum::<u32>() + floor
}

// Read by a function written after a call of it.
fn stepped(n: u32) -> u32 {
    let a = step(n);
    const STEP: u32 = 5;
    fn step(x: u32) -> u32 {
        x + STEP
    }
    a
}

// In a block, and a match's pattern.
fn priority(level: u32) -> &'static str {
    if level > 0 {
        const IMMEDIATE: u32 = 1;
        const NORMAL: u32 = 3;
        return match level {
            IMMEDIATE => "immediate",
            NORMAL => "normal",
            _ => "other",
        };
    }
    "none"
}

// Its own name apart from the module's `BASE`, which it reads too.
fn based(n: u32) -> u32 {
    const OWN_BASE: u32 = BASE * 2;
    n * OWN_BASE + BASE
}

// Named as a variable of the function is: the variable apart.
fn totalled(n: u32) -> u32 {
    #[cfg_attr(rust_js, rust_js::name = "total")]
    const TOTAL: u32 = 10;
    let total = n + TOTAL;
    total * 2
}

// Read by another constant: the module's.
fn scaled(n: u32) -> u32 {
    const UNIT: u32 = 3;
    const DOUBLE: u32 = UNIT * 2;
    n * DOUBLE + UNIT
}

// Read by a method of a type written in the body, which is the module's:
// the module's.
fn gauge(reading: u32) -> bool {
    const LIMIT: u32 = 9;
    struct Gauge(u32);
    impl Gauge {
        fn full(&self) -> bool {
            self.0 >= LIMIT
        }
    }
    Gauge(reading).full()
}

// In a closure's body, and a method's.
struct Meter {
    reading: u32,
}

impl Meter {
    fn over(&self) -> bool {
        const MAX: u32 = 100;
        self.reading > MAX
    }
}

fn labels(values: &[u32]) -> Vec<String> {
    values
        .iter()
        .map(|&v| {
            const WIDE: usize = 4;
            format!("{v:>WIDE$}")
        })
        .collect()
}

fn main() {
    println!("{} {}", includes_blocking_lane(2), includes_blocking_lane(4));
    println!("{} {}", is_blocking_lane(16), is_blocking_lane(8));
    println!("{:?}", clamped(&[1, 50]));
    println!("{}", stepped(1));
    println!("{}", capped(&[3, 9]));
    println!("{} {} {} {}", priority(1), priority(3), priority(2), priority(0));
    println!("{}", based(2));
    println!("{}", scaled(5));
    println!("{}", totalled(1));
    println!("{} {}", Meter { reading: 101 }.over(), Meter { reading: 3 }.over());
    println!("{} {}", gauge(9), gauge(2));
    println!("{:?}", labels(&[7, 42]));
}
