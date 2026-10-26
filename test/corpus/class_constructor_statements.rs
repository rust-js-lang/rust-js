// A class's constructor of statements, as React's `FiberNode` is (ADR
// 0761, amended): the variable it returns, made of a struct literal, is
// `this`, its fields set where it's made, a `None`'s left out; what follows
// sets fields on a flag and reads them, and an early return gives it too.
// What Rust reads of it is what it does natively.
use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Fiber {
    tag: u8,
    actual_duration: Cell<Option<f64>>,
    reads: Cell<u32>,
}

fn bump(count: &Cell<u32>) {
    count.set(count.get() + 1);
}

#[cfg_attr(rust_js, rust_js::class)]
impl Fiber {
    pub fn new(tag: u8, profiled: bool) -> Self {
        let fiber = Fiber {
            tag,
            actual_duration: Cell::new(None),
            reads: Cell::new(0),
        };
        if tag == 0 {
            return fiber;
        }
        if profiled {
            fiber.actual_duration.set(Some(-0.0));
        }
        for _ in 0..tag {
            bump(&fiber.reads);
        }
        fiber
    }

    pub fn shown(&self) -> String {
        format!("{} {:?} {}", self.tag, self.actual_duration.get(), self.reads.get())
    }
}

fn main() {
    for (tag, profiled) in [(0, true), (2, true), (3, false)] {
        println!("{}", Fiber::new(tag, profiled).shown());
    }
}
