// A function's last `match`, of a guarded arm falling to the `_`, whose
// middle arms are each an `if … else` that gives `()`: its `switch`, as
// React's accumulateSuspenseyCommitOnFiber writes it, `case 22: if (hidden)
// { // .. } else { .. } break;`, no `return;` MIR made, each comment in its
// own arm (rd93), the guarded case `if (ENABLE) { ..; break; }` falling
// through to the `default` (rd136).
use std::cell::Cell;

thread_local! {
    static ROOT: Cell<u32> = const { Cell::new(0) };
}

fn visit(n: u32) {
    ROOT.set(ROOT.get() + n);
}

const ENABLE: bool = true;

fn accumulate(tag: u8, hidden: bool, resources: bool, n: u32) {
    match tag {
        0 => {
            visit(n);
            if n > 3 {
                visit(1);
            }
        }
        3 | 4 => {
            if resources {
                let previous = ROOT.get();
                visit(n);
                ROOT.set(previous);
            } else {
                visit(n);
            }
        }
        22 => {
            if hidden {
                // Don't suspend in hidden trees
            } else {
                visit(n);
            }
        }
        30 if ENABLE => {
            if n > 1 {
                visit(2);
            }
            visit(n);
        }
        _ => {
            visit(n * 100);
        }
    }
}

/// Of a guarded arm a later arm goes on from, one that doesn't name its
/// constant, the `if` chain Rust tries (ADR 0629): `} else if (tag === 22)
/// { if (hidden) { // .. } else { .. } } else if (tag === 30 && ENABLE) {
/// .. } else if (tag === 40) { .. } else { .. }`, as written, no `return;`
/// MIR made (rd93, rd600).
fn accumulate_chain(tag: u8, hidden: bool, resources: bool, n: u32) {
    match tag {
        0 => {
            visit(n);
            if n > 3 {
                visit(1);
            }
        }
        3 | 4 => {
            if resources {
                let previous = ROOT.get();
                visit(n);
                ROOT.set(previous);
            } else {
                visit(n);
            }
        }
        22 => {
            if hidden {
                // Don't suspend in hidden trees
            } else {
                visit(n);
            }
        }
        30 if ENABLE => {
            if n > 1 {
                visit(2);
            }
            visit(n);
        }
        40 => visit(4),
        _ => {
            visit(n * 100);
        }
    }
}

/// Of an arm after it that names the guarded arm's constant too, where
/// the guard fails, that arm, as Rust tries it.
fn accumulate_again(tag: u8, n: u32) {
    match tag {
        0 => visit(n),
        30 if !ENABLE => visit(1),
        40 => visit(4),
        30 => visit(3),
        _ => {
            visit(n * 100);
        }
    }
}

fn main() {
    for tag in [0, 3, 4, 22, 30, 40, 7] {
        for (hidden, resources) in [(false, false), (true, true), (false, true)] {
            for n in [1, 5] {
                ROOT.set(0);
                accumulate(tag, hidden, resources, n);
                let switched = ROOT.replace(0);
                accumulate_chain(tag, hidden, resources, n);
                let chained = ROOT.replace(0);
                accumulate_again(tag, n);
                println!("{tag} {hidden} {resources} {n}: {switched} {chained} {}", ROOT.get());
            }
        }
    }
}
