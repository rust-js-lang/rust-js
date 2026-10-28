// A loop's `catch_unwind` whose closure returns early, its `Ok` arm leaving
// the loop: the closure's body is the `try`'s, each `return` the arm's
// jump, `break outer;` from inside a `switch`, as React's renderRootSync
// writes `outer: do { try { switch (..) { case X: ..; break outer; } ..;
// break; } catch (thrownValue) { .. } } while (true);` (ADR 0660, amended,
// the user's decision, wl4). Of a loop the program didn't label, a `return`
// a `switch` would take stays the closure's.
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static REASON: Cell<u8> = const { Cell::new(0) };
    static TRIES: Cell<u32> = const { Cell::new(0) };
}

fn work(n: u8) {
    TRIES.set(TRIES.get() + 1);
    if n == 9 && TRIES.get() < 3 {
        panic!("suspended");
    }
}

fn render(start: u8) -> u8 {
    REASON.set(start);
    TRIES.set(0);
    let mut status = 0;
    let mut thrown = 0;
    'outer: loop {
        match panic::catch_unwind(AssertUnwindSafe(|| {
            match REASON.get() {
                1 => {
                    status = 6;
                    return;
                }
                2 | 3 => {
                    work(REASON.get());
                    if TRIES.get() > 1 {
                        status = 7;
                        return;
                    }
                }
                _ => work(REASON.get()),
            }
            status = 1;
        })) {
            Ok(()) => break 'outer,
            Err(_) => {
                thrown += 1;
                REASON.set(2);
            }
        }
    }
    status * 10 + thrown
}

// No label the program wrote, and no `switch` around the `return`: the
// loop's own `continue`.
fn retried(limit: u32) -> u32 {
    let mut done = 0;
    let mut turns = 0;
    while turns < limit {
        turns += 1;
        match panic::catch_unwind(AssertUnwindSafe(|| {
            if turns % 2 == 0 {
                return;
            }
            work(9);
            done += 1;
        })) {
            Ok(()) => continue,
            Err(_) => done += 100,
        }
    }
    done
}

// No label, a `return` inside a `match` of constants: the closure stays.
fn kept(reason: u8) -> u8 {
    let mut status = 0;
    loop {
        match panic::catch_unwind(AssertUnwindSafe(|| match reason {
            1 => {
                status = 3;
            }
            2 | 4 => {
                status = 4;
            }
            _ => {
                if reason > 7 {
                    return;
                }
                status = 5;
            }
        })) {
            Ok(()) => break,
            Err(_) => status = 9,
        }
    }
    status
}

// A label the program wrote that nothing names once its `break` is the
// loop's own: none in the JS.
fn first_big(rows: &[u32]) -> u32 {
    let mut seen = 0;
    'scan: for &row in rows {
        if row > 10 {
            break 'scan;
        }
        seen += row;
    }
    seen
}

fn main() {
    for start in [0, 1, 2, 3, 9] {
        println!("{start}: {}", render(start));
    }
    TRIES.set(0);
    println!("{}", retried(5));
    println!("{} {} {}", kept(1), kept(4), kept(8));
    println!("{}", first_big(&[1, 2, 30, 4]));
}
