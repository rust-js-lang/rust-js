// A chain of guarded arms each falling to the next, beside another, is one
// `switch` whose cases fall through, each arm's body in its guard, `case
// HostSingleton: if (supportsSingletons) { ..; break; } // Fall through`,
// as React's commitDeletionEffectsOnFiber writes it, not the bodies MIR
// placed past the `match` in blocks `bb19` and `bb288` (ADR 0905, amended).
use std::cell::Cell;

const HOISTABLE: u32 = 26;
const SINGLETON: u32 = 27;
const COMPONENT: u32 = 5;
const TEXT: u32 = 6;
const PORTAL: u32 = 4;
const FRAGMENT: u32 = 7;
const VIEW: u32 = 30;

struct Fiber {
    tag: Cell<u32>,
    state: Cell<u32>,
}

thread_local! {
    static HIDDEN: Cell<bool> = const { Cell::new(false) };
    static PARENT: Cell<Option<u32>> = const { Cell::new(None) };
}

fn traverse(f: &Fiber, log: &mut Vec<u32>, at: u32) {
    log.push(at);
    f.state.set(f.state.get() + 1);
}

fn delete(f: &Fiber, resources: bool, singletons: bool, view: bool, log: &mut Vec<u32>) {
    let prev_start = f.state.get();
    match f.tag.get() {
        HOISTABLE if resources => {
            if !HIDDEN.get() {
                traverse(f, log, 1);
            }
            traverse(f, log, 2);
        }
        // Fall through
        HOISTABLE | SINGLETON if singletons => {
            if !HIDDEN.get() {
                traverse(f, log, 3);
            }
            let prev = PARENT.get();
            if f.state.get() > 3 {
                PARENT.set(Some(1));
            }
            traverse(f, log, 4);
            PARENT.set(prev);
        }
        // Fall through
        HOISTABLE | SINGLETON | COMPONENT | TEXT => {
            if f.tag.get() != TEXT {
                traverse(f, log, 5);
            }
            let prev = PARENT.get();
            PARENT.set(None);
            traverse(f, log, 6);
            PARENT.set(prev);
            if let Some(p) = PARENT.get() {
                log.push(p);
            }
        }
        PORTAL => traverse(f, log, 7),
        VIEW if view => traverse(f, log, 8),
        // Fallthrough
        VIEW | FRAGMENT => {
            if !HIDDEN.get() {
                traverse(f, log, 9);
            }
            traverse(f, log, 10);
        }
        _ => traverse(f, log, 11),
    }
    if HIDDEN.get() && f.state.get() > 2 {
        traverse(f, log, 12);
    }
    f.state.set(prev_start + 100);
}

// The same of plain numbers, each arm one statement.
fn named(tag: u32, a: bool, b: bool, c: bool, log: &mut Vec<u32>) {
    match tag {
        1 if a => log.push(1),
        // Fall through
        1 | 2 if b => log.push(2),
        // Fall through
        1 | 2 | 3 => log.push(3),
        4 if c => log.push(4),
        // Fall through
        4 | 5 => log.push(5),
        _ => log.push(6),
    }
    log.push(0);
}

fn main() {
    let mut log = Vec::new();
    for tag in [HOISTABLE, SINGLETON, COMPONENT, TEXT, PORTAL, FRAGMENT, VIEW, 99] {
        for flags in 0..8 {
            let (resources, singletons, view) = (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0);
            HIDDEN.set(flags == 5);
            PARENT.set((flags == 3).then_some(40));
            let f = Fiber {
                tag: Cell::new(tag),
                state: Cell::new(flags),
            };
            delete(&f, resources, singletons, view, &mut log);
            log.push(f.state.get());
        }
    }
    println!("{log:?}");
    let mut log = Vec::new();
    for tag in 0..7 {
        for flags in 0..8 {
            named(tag, flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, &mut log);
        }
    }
    println!("{log:?}");
}
