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
pub const K: u8 = 6;

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

fn main() {
    for (tag, legacy_off) in [(A, true), (A, false), (B, true), (C, false), (K, true), (7, false)] {
        let caught = panic::catch_unwind(|| complete(tag, legacy_off)).is_err();
        let on = complete_on(tag, legacy_off);
        let log = LOG.take();
        println!("{tag} {legacy_off}: panicked {caught}, {on:?}, {log:?}");
    }
}
