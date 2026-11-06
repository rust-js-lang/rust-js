// A guard whose drop may panic, so no `finally` holds it: each early
// return out of its scope drops it where it leaves, in a loop too, and a
// panic in the scope drops it as it unwinds (#847, #861).
use std::cell::Cell;
use std::panic;

thread_local! {
    static FAIL_AT: Cell<u32> = const { Cell::new(u32::MAX) };
}

fn check(at: u32) -> Option<u32> {
    if at == FAIL_AT.with(Cell::get) {
        panic!("failed at {at}");
    }
    Some(at)
}

struct Restore(&'static str);

impl Drop for Restore {
    fn drop(&mut self) {
        if self.0 == "panics" {
            panic!("restore panics");
        }
        println!("restored {}", self.0);
    }
}

fn run(owner: Option<u32>, n: u32) -> u32 {
    let _restore = Restore("run");
    let stack = check(n);
    if let Some(owner) = owner {
        if let Some(task) = check(owner) {
            println!("task {task}");
            return task;
        }
    }
    for i in 0..n {
        if check(i + 10) == Some(12) {
            println!("found {i}");
            return i;
        }
    }
    stack.unwrap_or(0)
}

fn main() {
    println!("{}", run(Some(3), 1));
    println!("{}", run(None, 5));
    println!("{}", run(None, 2));
    FAIL_AT.with(|f| f.set(11));
    let caught = panic::catch_unwind(|| run(None, 5));
    println!("caught {}", caught.is_err());
    FAIL_AT.with(|f| f.set(3));
    let caught = panic::catch_unwind(|| run(Some(3), 5));
    println!("caught {}", caught.is_err());
}
