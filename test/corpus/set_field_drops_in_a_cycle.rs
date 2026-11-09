// A set beside a value of the type it's inside: what the set's items drop
// is found as any type's is, so the value's destructor still runs where
// its holder's drop drops it. A set of a type no other part holds on each
// side of the field that reaches back, so whichever order the drop glue's
// parts are walked in, a set's walk comes after it and keeps what it
// reached (#917).
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
    tags: HashSet<String>,
    next: Option<Box<Task>>,
    seen: BTreeSet<Vec<u32>>,
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
