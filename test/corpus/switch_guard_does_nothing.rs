// A guarded arm that does nothing, `A if legacy_off => {}`, before an arm
// naming its constant and more, as React's completeWork writes `case
// IncompleteFunctionComponent: { if (disableLegacyMode) { break; } //
// Fallthrough }`: where its guard holds, it goes past the `match`, to the
// panic after it, never to the next arm's body, which MIR placed after the
// `switch` (ADR 0629, amended).
use std::cell::RefCell;
use std::panic;

pub const A: u8 = 1;
pub const B: u8 = 2;
pub const C: u8 = 3;
pub const H: u8 = 4;
pub const S: u8 = 5;
pub const K: u8 = 6;
pub const I: u8 = 7;
pub const SC: u8 = 8;
pub const T: u8 = 9;

thread_local! {
    static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn work(n: u8) {
    LOG.with_borrow_mut(|log| log.push(n));
}

fn complete(tag: u8, legacy_off: bool) -> Option<u8> {
    match tag {
        A if legacy_off => {}
        // Fallthrough
        A | B | C => {
            work(1);
            return None;
        }
        K => {
            work(4);
            return None;
        }
        _ => {}
    }
    panic!("Unknown unit of work tag ({tag}).");
}

fn complete_on(tag: u8, legacy_off: bool) -> Option<u8> {
    match tag {
        A if legacy_off => {}
        // Fallthrough
        A | B | C => {
            work(1);
            return None;
        }
        K => {
            work(4);
            return None;
        }
        _ => {}
    }
    work(9);
    Some(tag)
}

/// React's completeWork's whole shape: guards falling through to the next
/// arm, and `I if legacy_off => {}` before `I => { .. }`, one case of them,
/// `case I: if (legacyOff) { break; } ..`.
fn complete_all(tag: u8, legacy_off: bool, resources: bool, singletons: bool, scope: bool) -> Option<u8> {
    match tag {
        A if legacy_off => {}
        // Fallthrough
        A | B | C => return Some(1),
        H if resources => return Some(2),
        // Fall through
        H | S if singletons => return Some(3),
        // Fall through
        H | S | K => return Some(4),
        I if legacy_off => {}
        I => {
            work(5);
            return Some(5);
        }
        SC if scope => return Some(6),
        // Only Legacy Mode completes an errored node.
        T if !legacy_off => return Some(7),
        _ => {}
    }
    panic!("Unknown unit of work tag ({tag}).");
}

fn main() {
    for (tag, legacy_off) in [(A, true), (A, false), (B, true), (C, false), (K, true), (7, false)] {
        let caught = panic::catch_unwind(|| complete(tag, legacy_off)).is_err();
        let on = complete_on(tag, legacy_off);
        let log = LOG.take();
        println!("{tag} {legacy_off}: panicked {caught}, {on:?}, {log:?}");
    }
    for tag in [A, B, H, S, K, I, SC, T, 10] {
        for flags in 0..16u8 {
            let [legacy_off, resources, singletons, scope] = [1, 2, 4, 8].map(|bit| flags & bit != 0);
            let done = panic::catch_unwind(|| complete_all(tag, legacy_off, resources, singletons, scope));
            let log = LOG.take();
            println!("all {tag} {flags}: {done:?}, {log:?}");
        }
    }
}
