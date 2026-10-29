// A function's last `match`, of a guarded arm, so an `if` chain, whose
// middle arms are each an `if … else` that gives `()`: the chain as
// written, `} else if (tag === 22) { if (hidden) { // .. } else { .. } }`,
// no `return;` MIR made, each comment in its own arm (rd93, React's
// accumulateSuspenseyCommitOnFiber).
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

fn main() {
    for tag in [0, 3, 4, 22, 30, 7] {
        for (hidden, resources) in [(false, false), (true, true), (false, true)] {
            for n in [1, 5] {
                ROOT.set(0);
                accumulate(tag, hidden, resources, n);
                println!("{tag} {hidden} {resources} {n}: {}", ROOT.get());
            }
        }
    }
}
