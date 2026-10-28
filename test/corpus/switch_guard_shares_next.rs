// A guarded arm that falls to the next, `B if on_b`, whose next names its
// constant and more, guarded and falling to nothing, `B | C if on_c`, then
// `_ => {}`: a `switch` whose `case B` falls through to `case C`, and
// what's written past the `match` after it, as React's
// attemptEarlyBailoutIfNoScheduledUpdate writes it (ADR 0905, amended).
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

const A: u8 = 1;
const B: u8 = 2;
const C: u8 = 3;

struct Fiber {
    tag: Cell<u8>,
    lanes: Cell<u32>,
}

fn bailout(fiber: &Fiber) -> u32 {
    fiber.lanes.get() * 10
}

fn update_c(fiber: &Fiber) -> u32 {
    fiber.lanes.get() + 100
}

fn early_bailout(fiber: &Fiber, on_b: bool, on_c: bool) -> u32 {
    // This fiber does not have any pending work.
    match fiber.tag.get() {
        A => {
            fiber.lanes.set(1);
        }
        B if on_b => {
            fiber.lanes.set(2);
        }
        // Fallthrough
        B | C if on_c => {
            fiber.lanes.set(3);
            return update_c(fiber);
        }
        _ => {}
    }
    bailout(fiber)
}

// A guard of `||`, `C if legacy || fiber.lanes.get() != 0`, is the `if`
// of its case, `case C: if (legacy || fiber.lanes !== 0) { .. }`, as
// React's throwException tests an Offscreen boundary.
fn captured(fiber: &Fiber, legacy: bool) -> bool {
    match fiber.tag.get() {
        A | B => {
            if legacy {
                fiber.lanes.set(0);
            }
            return false;
        }
        C if legacy || fiber.lanes.get() != 0 => {
            fiber.lanes.set(5);
            return false;
        }
        _ => {}
    }
    panic!("Unexpected tag ({})", fiber.tag.get());
}

// The same of a handler held in an `if let`, whose `match` tidy tests as
// `tag === C && (legacy || ..)`.
fn held(handler: Option<Rc<Fiber>>, legacy: bool) -> bool {
    if let Some(boundary) = handler {
        match boundary.tag.get() {
            A | B => {
                if legacy {
                    boundary.lanes.set(0);
                }
                return false;
            }
            C if legacy || boundary.lanes.get() != 0 => {
                boundary.lanes.set(5);
                return false;
            }
            _ => {}
        }
        panic!("Unexpected tag ({})", boundary.tag.get());
    }
    true
}

fn main() {
    for (tag, legacy, lanes) in [(A, false, 0), (C, true, 0), (C, false, 4), (C, false, 0)] {
        let fiber = Rc::new(Fiber {
            tag: Cell::new(tag),
            lanes: Cell::new(lanes),
        });
        let handler = Some(Rc::clone(&fiber));
        let caught = panic::catch_unwind(AssertUnwindSafe(|| held(handler, legacy)));
        print!("{:?}/{} ", caught.ok(), fiber.lanes.get());
    }
    println!("{}", held(None, false));
    for tag in [A, B, C] {
        for (legacy, lanes) in [(false, 0), (true, 0), (false, 4)] {
            let fiber = Fiber {
                tag: Cell::new(tag),
                lanes: Cell::new(lanes),
            };
            let caught = panic::catch_unwind(AssertUnwindSafe(|| captured(&fiber, legacy)));
            print!("{:?}/{} ", caught.ok(), fiber.lanes.get());
        }
    }
    println!();
    for tag in [A, B, C, 9] {
        for (on_b, on_c) in [(false, false), (true, false), (false, true), (true, true)] {
            let fiber = Fiber {
                tag: Cell::new(tag),
                lanes: Cell::new(7),
            };
            print!("{} ", early_bailout(&fiber, on_b, on_c));
        }
        println!();
    }
}
