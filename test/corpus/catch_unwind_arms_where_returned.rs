// A `catch_unwind` closure that returns literals, and `Ok` arms that test
// them, is each arm where the closure returns its value: React's
// throwAndUnwindWorkLoop, `try { const didFatal = throwException(..); if
// (didFatal) { panicOnRootError(..); return; } } catch (error) { .. }`
// (rd60, ADR 0706 amended). An arm that may panic stays after the `try`,
// so what it throws is never the closure's `Err`: one that panics, one
// that calls what panics. Numbers and chars, a `return` early of what
// does nothing, a `catch` that goes on, a value returned twice, a loop's
// `break`, and a name the closure and its arm both declare; a `return` in
// the closure's loop whose arm returns, one whose arm may `break` and one
// whose arm does nothing, each of which would be of the closure's loop.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

thread_local! {
    static LOG: Cell<u32> = const { Cell::new(0) };
    static WORK: Cell<Option<u32>> = const { Cell::new(None) };
}

fn note(n: u32) {
    LOG.set(LOG.get() * 10 + n % 10);
}

fn throw_exception(x: u32) -> bool {
    note(x);
    if x == 5 {
        panic!("exception {x}");
    }
    x > 2
}

fn panic_on_root_error(x: u32) {
    note(x + 1);
    if x >= 7 {
        panic!("root {x}");
    }
}

/// throwAndUnwindWorkLoop's.
fn unwind(x: u32, return_fiber: Option<u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        // Find and mark the nearest boundary.
        let did_fatal = throw_exception(x);
        if did_fatal {
            panic_on_root_error(x);
            return true;
        }
        // Not fatal.
        false
    })) {
        Ok(true) => return 0,
        Ok(false) => {}
        Err(error) => {
            // Bubble the error up to the next parent.
            if let Some(return_fiber) = return_fiber {
                WORK.set(Some(return_fiber));
                resume_unwind(error);
            } else {
                panic_on_root_error(x);
                return 1;
            }
        }
    }
    x + 1
}

/// An arm that panics, which the closure's `Err` never catches.
fn arm_panics(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if throw_exception(x) {
            return true;
        }
        note(4);
        false
    })) {
        Ok(true) => panic!("arm {x}"),
        Ok(false) => {}
        Err(_) => return 9,
    }
    x + 2
}

/// An arm that calls what panics.
fn arm_calls(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if throw_exception(x) {
            return true;
        }
        false
    })) {
        Ok(true) => {
            panic_on_root_error(x);
            return 2;
        }
        Ok(false) => {}
        Err(_) => return 9,
    }
    x + 3
}

/// Numbers, whose `catch` goes on.
fn numbers(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if throw_exception(x) {
            return 1;
        }
        if x == 2 {
            return 2;
        }
        0
    })) {
        Ok(1) => return 10,
        Ok(2) => {
            // Two is its own.
            return 20;
        }
        Ok(_) => {}
        Err(_) => note(3),
    }
    x + 4
}

/// Chars.
fn chars(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if throw_exception(x) {
            return 'a';
        }
        if x == 1 {
            return 'b';
        }
        'c'
    })) {
        Ok('a') => return 30,
        Ok('b') => return 31,
        Ok(_) => {}
        Err(_) => return 32,
    }
    x + 5
}

/// A `return` early of what does nothing, what follows it its `else`, and
/// an arm placed at the closure's last statement with what's said in it.
fn early(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if x == 0 {
            return false;
        }
        note(6);
        if !throw_exception(x) {
            return false;
        }
        // It's thrown.
        true
    })) {
        // Where it's thrown.
        Ok(true) => {
            // Forty, as it's thrown.
            return 40;
        }
        Ok(false) => {}
        Err(_) => return 41,
    }
    x + 6
}

/// A value returned twice, whose arm is more than one jump, which stays
/// the value set and tested, not its statements written twice.
fn twice(x: u32) -> u32 {
    let mut total = x;
    match catch_unwind(AssertUnwindSafe(|| {
        if x == 1 {
            return true;
        }
        if x == 2 {
            return false;
        }
        if throw_exception(x) {
            return true;
        }
        false
    })) {
        Ok(true) => {
            total += 50;
            return total;
        }
        Ok(false) => total += 1,
        Err(_) => return 51,
    }
    total + 7
}

/// A loop's `break`, and a name the closure declares that its arm reads
/// of its own.
fn looped(xs: &[u32]) -> u32 {
    let mut seen = 0;
    for &x in xs {
        let total = x * 2;
        match catch_unwind(AssertUnwindSafe(|| {
            let total = x + 100;
            if throw_exception(total - 100) {
                return true;
            }
            false
        })) {
            Ok(true) => break,
            Ok(false) => {}
            Err(_) => return 60 + seen,
        }
        seen += total;
    }
    seen
}

/// A `return` in the closure's loop, whose arm returns from the function,
/// as the Flight client's fulfillReference returns.
fn searched(xs: &[u32]) -> u32 {
    let mut found = 0;
    for &limit in xs {
        match catch_unwind(AssertUnwindSafe(|| {
            for &x in xs {
                if x == limit {
                    continue;
                }
                if throw_exception(x) && x > limit {
                    return false;
                }
            }
            true
        })) {
            Ok(true) => found += 1,
            Ok(false) => return 70 + found,
            Err(_) => break,
        }
    }
    found
}

/// A `return` in the closure's loop whose arm may `break` the loop around
/// the `match`, which would be of the closure's loop: the value stays set
/// and tested.
fn broke(xs: &[u32]) -> u32 {
    let mut seen = 0;
    for &limit in xs {
        match catch_unwind(AssertUnwindSafe(|| {
            for &x in xs {
                if throw_exception(x) && x > limit {
                    return true;
                }
            }
            false
        })) {
            Ok(true) => {
                if limit > 3 {
                    break;
                }
                seen *= 3;
                return 90 + seen;
            }
            Ok(false) => {}
            Err(_) => return 80,
        }
        seen += limit;
    }
    seen
}

/// A `return` in the closure's loop whose arm does nothing, which would go
/// on with the closure's loop: the value stays set and tested.
fn kept_on(xs: &[u32]) -> u32 {
    let mut seen = 0;
    for &limit in xs {
        match catch_unwind(AssertUnwindSafe(|| {
            for &x in xs {
                if throw_exception(x) && x > limit {
                    return false;
                }
            }
            true
        })) {
            Ok(true) => return 100 + seen,
            Ok(false) => {}
            Err(_) => return 101,
        }
        seen += limit;
    }
    seen
}

fn shown(f: impl FnOnce() -> u32) -> String {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v.to_string(),
        Err(payload) => match payload.downcast_ref::<String>() {
            Some(text) => format!("panic {text}"),
            None => "panic".to_string(),
        },
    }
}

fn main() {
    for x in [0, 1, 2, 3, 5, 8] {
        LOG.set(0);
        WORK.set(None);
        let values = [
            shown(|| unwind(x, None)),
            shown(|| unwind(x, Some(4))),
            shown(|| arm_panics(x)),
            shown(|| arm_calls(x)),
            shown(|| numbers(x)),
            shown(|| chars(x)),
            shown(|| early(x)),
            shown(|| twice(x)),
        ];
        println!("{x} {values:?} {} {:?}", LOG.get(), WORK.get());
    }
    LOG.set(0);
    println!(
        "looped {} {} {} {}",
        looped(&[1, 2]),
        looped(&[1, 3, 1]),
        looped(&[2, 5]),
        LOG.get()
    );
    LOG.set(0);
    println!(
        "searched {} {} {} {}",
        searched(&[1, 2]),
        searched(&[4, 3, 1]),
        searched(&[2, 5, 9]),
        LOG.get()
    );
    LOG.set(0);
    println!(
        "broke {} {} {} {}",
        broke(&[4, 6]),
        broke(&[4, 3, 1]),
        broke(&[2, 5]),
        LOG.get()
    );
    LOG.set(0);
    println!(
        "kept on {} {} {} {}",
        kept_on(&[1, 2]),
        kept_on(&[4, 3]),
        kept_on(&[2, 5]),
        LOG.get()
    );
}
