// A function whose `Drop` guard is a `try { .. } finally { .. }` returns
// early from inside an `if` as it's written, `if (task != null) {
// run(task); return; }` in the `try`, whose `finally` still runs: no block
// MIR made, `bb18: { .. break bb18; }`, around it (rd502). React's Flight
// client's replayConsoleWithCallStack.
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

fn main() {
    replay(Some(5), None);
    replay(Some(1), Some(3));
    replay(None, None);
    println!("depth {}", DEPTH.get());
}
