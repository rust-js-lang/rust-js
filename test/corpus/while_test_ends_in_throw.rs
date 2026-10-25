// A written `while` whose body may end by panicking, a division by a
// divisor it reads, with an `Rc` it drops: its test is the `while`'s, and
// nothing follows the panic's `throw` (#290, fuzz seed 61706).

use std::cell::RefCell;
use std::rc::Rc;

fn id<T>(x: T) -> T {
    x
}

#[derive(Clone, Copy)]
struct Span {
    from: i32,
    to: u8,
    width: i64,
}

impl Span {
    fn from(&self) -> i32 {
        self.from
    }
}

// The loop ends its function: what leaves it is the function's end.
fn spans(round: u32) {
    let mut span = Span {
        from: 1,
        to: 2,
        width: 3,
    };
    while round < id(2u32) && [id(241u64)].last().copied().is_none() {
        println!("{} {} {}", span.from, span.to, span.width);
        let shared = Rc::new(RefCell::new(id(-2i64)));
        let next = Span {
            from: 1,
            to: 1,
            width: (Rc::strong_count(&shared) as i64) << (round / id(2u32)),
        };
        span = Span {
            from: next.from(),
            to: 1,
            width: 2,
        };
    }
}

fn main() {
    spans(id(0));
    println!("end");
}
