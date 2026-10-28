// Arms of an `if` chain that each end in a `continue`, with no `else`,
// each its own `continue`, as written, where MIR shares one: no labeled
// block whose `break` is the chain's fall, as React's
// commitBeforeMutationEffects_begin walks its fibers (ADR 0702, amended).
use std::cell::Cell;

struct Fiber {
    lanes: Cell<u32>,
    log: Cell<u32>,
}

fn complete(fiber: &Fiber) {
    fiber.lanes.set(fiber.lanes.get() - 1);
    fiber.log.set(fiber.log.get() * 3 + fiber.lanes.get() % 3);
}

fn walk(fiber: &Fiber, modern: bool) {
    while fiber.lanes.get() > 0 {
        if modern {
            if fiber.lanes.get() == 5 {
                complete(fiber);
                continue;
            } else if fiber.lanes.get() == 7 {
                fiber.lanes.set(6);
                complete(fiber);
                continue;
            }
        }
        complete(fiber);
        fiber.log.set(fiber.log.get() + 1);
    }
}

fn first_even(items: &[u32], skip_odd: bool) -> Option<u32> {
    for &item in items {
        if skip_odd {
            if item % 2 == 1 {
                continue;
            } else if item == 4 {
                break;
            } else if item > 10 {
                continue;
            }
        }
        if item % 2 == 0 {
            return Some(item);
        }
    }
    None
}

fn main() {
    for (lanes, modern) in [(9, true), (9, false), (5, true), (0, true)] {
        let fiber = Fiber {
            lanes: Cell::new(lanes),
            log: Cell::new(0),
        };
        walk(&fiber, modern);
        println!("{} {}", fiber.lanes.get(), fiber.log.get());
    }
    println!("{:?} {:?}", first_even(&[3, 12, 6], true), first_even(&[3, 4, 6], true));
    println!("{:?} {:?}", first_even(&[3, 12, 6], false), first_even(&[1, 3], true));
}
