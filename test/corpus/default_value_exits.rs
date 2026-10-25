// A value some arms of a `match` give, which tidy gives first as a default
// where no other way out of the match can tell (ADR 0364, amended): not
// where an arm that does nothing goes on past it, nor one that leaves the
// loop, the closure or the `catch_unwind` around it, where the variable is
// read; nor where a call before reads it, through a closure. React's
// `mapIntoArray` set `invokeCallback = true` before testing `$$typeof`,
// so every object called back (issue #219).
use std::panic::{self, AssertUnwindSafe};

// Some arms set the flag, one returns, the rest do nothing.
fn kind(value: Option<(&str, u32, u32)>) -> u32 {
    let mut invoke = false;
    match value {
        None => invoke = true,
        Some((kind, a, b)) => match kind {
            "string" => invoke = true,
            "object" => match (a, b) {
                (1, _) | (_, 1) => invoke = true,
                (2, 2) => return 2,
                _ => {}
            },
            _ => {}
        },
    }
    if invoke {
        return 1;
    }
    0
}

// A value made `None` just before, which the arm that does nothing keeps.
fn found(pairs: &[(u32, u32)]) -> Vec<Option<u32>> {
    let mut all = Vec::new();
    for &(a, b) in pairs {
        let mut seen = None;
        match (a, b) {
            (1, _) | (_, 1) => seen = Some(7),
            (2, 2) => break,
            _ => {}
        }
        all.push(seen);
    }
    all
}

// Another arm leaves the loop: the flag is read after it.
fn broken(pairs: &[(u32, u32)]) -> bool {
    let mut hit = false;
    for &(a, b) in pairs {
        match (a, b) {
            (1, _) | (_, 1) => hit = false,
            (2, 2) => break,
            _ => hit = true,
        }
    }
    hit
}

// Another arm goes on to the loop's next turn, which reads the flag.
fn continued(pairs: &[(u32, u32)]) -> Vec<bool> {
    let mut seen = Vec::new();
    let mut hit = false;
    for &(a, b) in pairs {
        seen.push(hit);
        match (a, b) {
            (1, _) | (_, 1) => hit = true,
            (2, 2) => continue,
            _ => hit = false,
        }
    }
    seen.push(hit);
    seen
}

// Another arm returns from the closure that sets the flag it captures.
fn returned(pairs: &[(u32, u32)]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let mut step = || {
            match (a, b) {
                (1, _) | (_, 1) => hit = true,
                (2, 2) => return,
                _ => hit = false,
            }
            println!("stepped {a} {b}");
        };
        step();
        seen.push(hit);
    }
    seen
}

// Another arm panics, inside the `catch_unwind` the flag is read after.
fn unwound(pairs: &[(u32, u32)]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let caught = panic::catch_unwind(AssertUnwindSafe(|| {
            match (a, b) {
                (1, _) | (_, 1) => hit = true,
                (2, 2) => panic!("two twos"),
                _ => hit = false,
            }
            println!("matched {a} {b}");
        }));
        seen.push(hit && caught.is_ok());
        seen.push(hit);
    }
    seen
}

// A call made before the value is given reads it, through a closure.
fn observed(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let mut level = 0;
        let look = || level * 10;
        match (a, b) {
            (1, _) | (_, 1) => level = 1,
            (2, 2) => level = 2,
            _ => {
                seen.push(look());
                level = 3;
            }
        }
        seen.push(level);
    }
    seen
}

// A guard's call before the value is given reads it, through a closure.
fn guarded(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let mut level = 4;
        let below = |n: u32| n < level;
        match (a, b) {
            (1, _) | (_, 1) => level = 1,
            (2, 2) => level = 2,
            (n, _) if below(n) => level = 3,
            _ => level = 5,
        }
        seen.push(level);
    }
    seen
}

fn half(n: i32) -> Option<i32> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

// A value a `let` is given, which nothing sees before: still given first.
fn chained(n: i32, calls: &mut u32) -> i32 {
    let v = if let Some(h) = half(n)
        && h != 0
        && let Some(q) = {
            *calls += 1;
            half(h)
        }
        && q > 1
    {
        q
    } else {
        0
    };
    v * 10
}

fn main() {
    let values = [
        None,
        Some(("string", 0, 0)),
        Some(("object", 1, 0)),
        Some(("object", 0, 1)),
        Some(("object", 2, 2)),
        Some(("object", 3, 3)),
        Some(("number", 1, 1)),
    ];
    for value in values {
        println!("{:?} {}", value, kind(value));
    }
    let pairs = [(1, 0), (2, 2), (0, 1), (3, 3), (2, 2), (5, 5)];
    println!("{:?}", found(&[(1, 0), (3, 3), (0, 1), (2, 2), (1, 1)]));
    println!("{}", broken(&pairs));
    println!("{}", broken(&[(3, 3), (2, 2), (1, 1)]));
    println!("{:?}", continued(&pairs));
    println!("{:?}", returned(&pairs));
    println!("{:?}", unwound(&[(3, 3), (2, 2), (1, 0), (2, 2)]));
    println!("{:?}", observed(&pairs));
    println!("{:?}", guarded(&[(1, 0), (2, 2), (3, 3), (5, 5), (0, 7)]));
    let mut calls = 0;
    let made: Vec<i32> = [12, 4, 3, 0, 8].iter().map(|&n| chained(n, &mut calls)).collect();
    println!("{made:?} {calls}");
}
