// A `loop` each of whose ways ends advancing to the next item where there
// is one and it isn't the first, else `break`, is a `do { .. } while`, as
// React's updateReducerImpl walks its circular queue (ADR 0978). Not where
// the variable is read after it, a way ends otherwise, or the test reads
// the variable apart from what it's given. A way may advance inside an
// `if` and go on, its other path falling to the loop's last advance, as
// updateReducerImpl skips an update: `update = update.next; continue;`
// (ADR 0978, amended); not beside another `continue`, nor of another test.
use std::cell::RefCell;
use std::rc::Rc;

struct Update {
    lane: u32,
    next: RefCell<Option<Rc<Update>>>,
}

fn ring(lanes: &[u32]) -> Rc<Update> {
    let nodes: Vec<Rc<Update>> = lanes
        .iter()
        .map(|&lane| {
            Rc::new(Update {
                lane,
                next: RefCell::new(None),
            })
        })
        .collect();
    for (i, node) in nodes.iter().enumerate() {
        *node.next.borrow_mut() = Some(Rc::clone(&nodes[(i + 1) % nodes.len()]));
    }
    Rc::clone(&nodes[0])
}

fn sum(first: &Rc<Update>, skip: u32) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn last_lane(first: &Rc<Update>) -> u32 {
    let mut update = Rc::clone(first);
    loop {
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    update.lane
}

fn below(first: &Rc<Update>, limit: u32) -> u32 {
    let mut count = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane >= limit {
            break;
        }
        count += 1;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    count
}

fn rising(first: &Rc<Update>) -> u32 {
    let mut steps = 0;
    let mut update = Rc::clone(first);
    loop {
        steps += 1;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if next.lane > update.lane => update = next,
            _ => break,
        }
    }
    steps
}

fn sum_then_last(first: &Rc<Update>, skip: u32) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total + update.lane
}

fn tested_apart(first: &Rc<Update>, skip: u32) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if next.lane != skip + 2 => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn rising_sum(first: &Rc<Update>, skip: u32) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if next.lane > update.lane => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if next.lane > update.lane => update = next,
            _ => break,
        }
    }
    total
}

fn capped(first: &Rc<Update>, skip: u32, cap: u32) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        if total > cap {
            break;
        }
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn skipping(first: &Rc<Update>, skip: u32, flag: bool) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        let mut should_skip = update.lane == skip;
        if flag && update.lane == 3 {
            if update.lane > 1 {
                // Clean up.
                let next = update.next.borrow().clone();
                match next {
                    Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                    _ => break,
                }
                continue;
            } else {
                should_skip = true;
            }
        }
        if should_skip {
            total += 100;
        } else {
            if update.lane == 4 {
                if flag {
                    total += 1000;
                }
                let next = update.next.borrow().clone();
                match next {
                    Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                    _ => break,
                }
                continue;
            }
            total += update.lane;
        }
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn skipping_once(first: &Rc<Update>, skip: u32) -> u32 {
    let mut total = 0;
    let mut seen = false;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == skip && !seen {
            seen = true;
            continue;
        }
        if update.lane == 2 {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if !Rc::ptr_eq(&next, first) => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn skipping_low(first: &Rc<Update>) -> u32 {
    let mut total = 0;
    let mut update = Rc::clone(first);
    loop {
        if update.lane == 2 {
            let next = update.next.borrow().clone();
            match next {
                Some(next) if next.lane > 1 => update = next,
                _ => break,
            }
            continue;
        }
        total += update.lane;
        let next = update.next.borrow().clone();
        match next {
            Some(next) if !Rc::ptr_eq(&next, first) => update = next,
            _ => break,
        }
    }
    total
}

fn main() {
    let first = ring(&[1, 2, 3, 4]);
    println!("{} {}", sum(&first, 2), sum(&first, 9));
    println!("{}", last_lane(&first));
    println!("{} {}", below(&first, 3), below(&first, 9));
    println!("{}", rising(&first));
    println!("{} {} {} {}", sum_then_last(&first, 2), tested_apart(&first, 1), rising_sum(&first, 1), capped(&first, 2, 4));
    let lone = ring(&[7]);
    println!("{} {}", sum(&lone, 0), rising(&lone));
    println!("{} {} {}", skipping(&first, 2, true), skipping(&first, 9, false), skipping(&lone, 7, true));
    println!("{} {}", skipping_once(&first, 3), skipping_low(&first));
}
