// A set beside a value of the type it's inside: what the set's items drop
// is found as any type's is, so the value's destructor still runs where
// its holder's drop drops it.
use std::collections::{BTreeSet, HashSet};

struct Task {
    name: &'static str,
    step: Step,
}

impl Drop for Task {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

struct Step {
    next: Option<Box<Task>>,
    tags: HashSet<String>,
    seen: BTreeSet<String>,
}

fn step(next: Option<Box<Task>>) -> Step {
    Step {
        next,
        tags: HashSet::new(),
        seen: BTreeSet::new(),
    }
}

fn main() {
    let first = Task {
        name: "first",
        step: step(None),
    };
    let held = step(Some(Box::new(Task {
        name: "inner",
        step: step(None),
    })));
    println!("{} {} {}", first.step.tags.len(), held.seen.len(), held.next.is_some());
    drop(held);
    println!("held dropped");
}
