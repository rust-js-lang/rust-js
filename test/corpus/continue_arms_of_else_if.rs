// Each arm of an `else if` chain that ends in a `continue` the program
// wrote keeps it, and the chain's fall-through falls out of the `if`, as
// React's commitBeforeMutationEffects_begin writes it, `if (isHidden) {
// ..; continue; } else if (..) { ..; continue; }`, not the `continue`s
// MIR joined after the chain and an `else { break bb107; }` past them
// (ADR 0745, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fiber {
    lanes: Cell<u32>,
    alternate: RefCell<Option<Rc<Fiber>>>,
    child: RefCell<Option<Rc<Fiber>>>,
}

thread_local! {
    static NEXT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

fn complete(log: &mut Vec<u32>, at: u32) {
    log.push(at);
    NEXT.set(None);
}

fn walk(eligible: bool, modern: bool, log: &mut Vec<u32>) {
    while let Some(fiber) = NEXT.with_borrow(Clone::clone) {
        if fiber.lanes.get() == 3 {
            complete(log, 1);
            continue;
        }
        if fiber.lanes.get() == 4 && modern {
            let current = fiber.alternate.borrow().clone();
            let is_hidden = fiber.child.borrow().is_none();
            if is_hidden {
                if let Some(current) = &current
                    && current.lanes.get() == 0
                    && eligible
                {
                    current.lanes.set(1);
                }
                complete(log, 2);
                continue;
            } else if current.as_ref().is_some_and(|c| c.lanes.get() == 7) {
                if eligible {
                    fiber.lanes.set(6);
                }
                complete(log, 3);
                continue;
            }
        }
        let child = fiber.child.borrow().clone();
        if fiber.lanes.get() != 0
            && let Some(child) = child
        {
            log.push(4);
            NEXT.set(Some(child));
        } else {
            complete(log, 5);
        }
    }
}

// The same chain where one arm returns: the `continue` in the arm that
// goes on, none after the `return`.
fn first_hidden(fibers: &[Rc<Fiber>]) -> Option<u32> {
    let mut seen = 0;
    for fiber in fibers {
        if fiber.lanes.get() == 9 {
            return Some(seen);
        } else if fiber.lanes.get() > 4 {
            seen += fiber.lanes.get();
            continue;
        } else if fiber.lanes.get() == 2 {
            seen += 100;
            continue;
        }
        seen += 1;
    }
    None
}

fn fiber(lanes: u32, alternate: Option<Rc<Fiber>>, child: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        lanes: Cell::new(lanes),
        alternate: RefCell::new(alternate),
        child: RefCell::new(child),
    })
}

fn main() {
    let mut log = Vec::new();
    for (lanes, alternate, child, eligible, modern) in [
        (3, None, None, true, true),
        (4, Some(0), None, true, true),
        (4, Some(0), None, false, true),
        (4, Some(7), Some(1), true, true),
        (4, Some(7), Some(1), false, true),
        (4, Some(2), Some(0), true, true),
        (4, None, Some(1), true, false),
        (0, None, None, true, true),
    ] {
        let alternate = alternate.map(|lanes| fiber(lanes, None, None));
        let child = child.map(|lanes| fiber(lanes, None, None));
        let top = fiber(lanes, alternate.clone(), child);
        NEXT.set(Some(Rc::clone(&top)));
        walk(eligible, modern, &mut log);
        log.push(top.lanes.get() * 10 + alternate.map_or(9, |a| a.lanes.get()));
    }
    println!("{log:?}");
    let fibers = [5, 2, 1, 6].map(|lanes| fiber(lanes, None, None));
    println!("{:?}", first_hidden(&fibers));
    let fibers = [1, 2, 9, 6].map(|lanes| fiber(lanes, None, None));
    println!("{:?}", first_hidden(&fibers));
}
