// A `match` of two arms is an `if` with both its branches, its last arm the
// `else`, as React's updateActivityComponent writes `if (current === null) {
// ..; return ..; } else { .. }` (ADR 0237, amended, the user's decision); a
// value's at a function's end is `return c ? a : b`, as a written if/else's
// value is (ADR 0879).
use std::cell::Cell;

struct Node {
    lanes: Cell<u32>,
}

fn tail_match(current: Option<&Node>, wip: &Node) -> u32 {
    match current {
        None => {
            if wip.lanes.get() == 1 {
                return 0;
            }
            wip.lanes.get()
        }
        Some(current) => {
            wip.lanes.set(current.lanes.get());
            current.lanes.get()
        }
    }
}

fn statement_match(current: Option<&Node>, wip: &Node) {
    match current {
        None => {
            wip.lanes.set(7);
            println!("none {}", wip.lanes.get());
        }
        Some(current) => {
            current.lanes.set(wip.lanes.get() + 1);
            println!("some {}", current.lanes.get());
        }
    }
}

fn value_match(n: Option<u32>) -> u32 {
    match n {
        Some(n) => n * 2,
        None => 1,
    }
}

// An `if`'s written `else` whose test MIR turned, `!flag`, stays too.
fn turned(flag: bool, v: &mut Vec<u32>) -> usize {
    if !flag {
        v.push(1);
        return 1;
    } else {
        v.push(2);
    }
    v.len()
}

// And of an `&&`, which preparation makes again as a test.
fn anded(on: bool, mode: u32, v: &mut Vec<u32>) -> bool {
    if on && (mode > 3 || mode & 1 != 0) {
        v.push(mode);
        return true;
    } else {
        v.push(0);
    }
    v.push(9);
    false
}

// A lone jump's arm stays a guard's way out, as `let … else` writes it.
fn summed(items: &[Option<u32>]) -> u32 {
    let mut sum = 0;
    let mut i = 0;
    loop {
        let item = match items.get(i) {
            Some(item) => item,
            None => break,
        };
        sum += item.unwrap_or(10);
        i += 1;
    }
    sum
}

fn main() {
    let wip = Node { lanes: Cell::new(1) };
    let other = Node { lanes: Cell::new(4) };
    println!("{}", tail_match(None, &wip));
    wip.lanes.set(3);
    println!("{}", tail_match(None, &wip));
    println!("{} {}", tail_match(Some(&other), &wip), wip.lanes.get());
    statement_match(None, &wip);
    statement_match(Some(&other), &wip);
    println!("{} {}", value_match(Some(4)), value_match(None));
    println!("{}", summed(&[Some(1), None, Some(5)]));
    let mut v = Vec::new();
    println!("{} {} {:?}", turned(true, &mut v), turned(false, &mut v), v);
    println!("{} {} {} {:?}", anded(true, 1, &mut v), anded(true, 2, &mut v), anded(false, 5, &mut v), v);
}
