// A function whose `Drop` guard is a `try { .. } finally { .. }` returns
// early from inside an `if` as it's written, `if (task != null) {
// run(task); return; }` in the `try`, whose `finally` still runs: no block
// MIR made, `bb18: { .. break bb18; }`, around it (rd502). React's Flight
// client's replayConsoleWithCallStack. Its last `if let`'s `return` as
// written, `if (root != null) { run(root); return; } run(0);`, one with an
// `else`, its `if .. else`, and a `let .. else` after an early `return`,
// each `if` its `return`, no `else` around the rest.
use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        DEPTH.set(self.0);
    }
}

fn task_of(n: u32) -> Option<u32> {
    if n > 1 { Some(n) } else { None }
}

fn run(n: u32) {
    println!("run {n} at depth {}", DEPTH.get());
}

pub fn replay(owner: Option<u32>, root: Option<u32>) {
    let _restore = Restore(DEPTH.get());
    DEPTH.set(7);
    if let Some(owner) = owner {
        let task = task_of(owner);
        if let Some(task) = task {
            run(task);
            return;
        }
    }
    if let Some(root) = root {
        run(root);
        return;
    }
    run(0);
}

pub fn replay_else(owner: Option<u32>, root: Option<u32>) {
    let _restore = Restore(DEPTH.get());
    DEPTH.set(8);
    if let Some(owner) = owner
        && owner > 3
    {
        run(owner);
        return;
    }
    if let Some(root) = root {
        run(root);
    } else {
        run(0);
    }
}

pub fn replay_let_else(owner: Option<u32>, root: Option<u32>) {
    let _restore = Restore(DEPTH.get());
    DEPTH.set(9);
    if let Some(owner) = owner
        && owner > 3
    {
        run(owner);
        return;
    }
    let Some(root) = root else {
        run(0);
        return;
    };
    run(root);
}

fn main() {
    replay(Some(5), None);
    replay(Some(1), Some(3));
    replay(None, None);
    replay_else(Some(5), None);
    replay_else(Some(1), Some(3));
    replay_else(None, None);
    replay_let_else(Some(5), None);
    replay_let_else(Some(1), Some(3));
    replay_let_else(None, None);
    println!("depth {}", DEPTH.get());
}
