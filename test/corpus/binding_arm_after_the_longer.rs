// A `match` of two arms whose chain would print the arm that only binds
// first, `Some(stacks) if .. => Some(stacks)`, and the longer `_` arm in
// the `else`: the longer first, of the test negated, `if (stacks$1 == null
// || stacks$1 === 0) { stacks = 7; .. } else { stacks = stacks$1; }`, as
// React's renderFunctionComponent writes `if (!stacks) { .. } else { ..
// }` (#774, the user's decision). An arm that does more than bind keeps
// its place, as does one that sets a field or gives a constant, and one
// whose other arm is no longer.
use std::cell::Cell;

fn present(n: Option<u32>) -> bool {
    n.is_some()
}

pub fn pure_guard(t: &Cell<Option<u32>>, dev: bool) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if stacks != 0 => Some(stacks),
            _ => {
                let stacks = 7;
                t.set(Some(stacks));
                Some(stacks)
            }
        }
    } else {
        None
    };
    stacks
}

pub fn calling_guard(t: &Cell<Option<u32>>, dev: bool) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if present(Some(stacks)) => Some(stacks),
            _ => {
                let stacks = 7;
                t.set(Some(stacks));
                Some(stacks)
            }
        }
    } else {
        None
    };
    stacks
}

pub fn doing_more(t: &Cell<Option<u32>>, dev: bool, log: &mut Vec<u32>) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if stacks != 0 => {
                log.push(stacks);
                Some(stacks)
            }
            _ => {
                let stacks = 7;
                t.set(Some(stacks));
                Some(stacks)
            }
        }
    } else {
        None
    };
    stacks
}

pub fn no_longer(t: &Cell<Option<u32>>, dev: bool) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if stacks != 0 => Some(stacks),
            _ => {
                t.set(None);
                Some(8)
            }
        }
    } else {
        None
    };
    stacks
}

pub fn constant(t: &Cell<Option<u32>>, dev: bool) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if stacks != 0 => Some(1),
            _ => {
                t.set(None);
                t.set(Some(2));
                Some(8)
            }
        }
    } else {
        None
    };
    stacks
}

pub fn setting(t: &Cell<Option<u32>>, last: &Cell<u32>, dev: bool) -> Option<u32> {
    let stacks: Option<u32> = if dev {
        match t.get() {
            Some(stacks) if stacks != 0 => {
                last.set(stacks);
                Some(stacks)
            }
            _ => {
                let stacks = 7;
                t.set(Some(stacks));
                last.set(stacks + 1);
                last.set(last.get() * 2);
                Some(stacks)
            }
        }
    } else {
        None
    };
    stacks
}

fn main() {
    let mut log = Vec::new();
    for given in [None, Some(0), Some(3)] {
        for dev in [false, true] {
            let t = Cell::new(given);
            println!("{:?} {:?}", pure_guard(&t, dev), t.get());
            let t = Cell::new(given);
            println!("{:?} {:?}", calling_guard(&t, dev), t.get());
            let t = Cell::new(given);
            println!("{:?} {:?}", doing_more(&t, dev, &mut log), t.get());
            let t = Cell::new(given);
            println!("{:?} {:?}", no_longer(&t, dev), t.get());
            let t = Cell::new(given);
            println!("{:?} {:?}", constant(&t, dev), t.get());
            let (t, last) = (Cell::new(given), Cell::new(0));
            let set = setting(&t, &last, dev);
            println!("{set:?} {:?} {}", t.get(), last.get());
        }
    }
    println!("{log:?}");
}
