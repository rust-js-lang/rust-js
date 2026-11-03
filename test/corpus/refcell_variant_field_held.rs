//@ run-fail: RefCell already mutably borrowed
// An enum variant's `RefCell` field a pattern binds, given on: the
// binding lends the field, as `&x.f` does, so a borrow held of what it's
// given while the field is borrowed again, through a binding of its own,
// finds it borrowed, as std's does (ADR 0863, amended, #503).
use std::cell::RefCell;

pub enum Job {
    Queued { tries: RefCell<u32> },
    Done,
}

pub struct Task {
    pub job: Job,
}

fn tries(task: &Task) -> u32 {
    match &task.job {
        Job::Queued { tries } => *tries.borrow(),
        Job::Done => 0,
    }
}

fn bump(cell: &RefCell<u32>, task: &Task) -> u32 {
    let mut held = cell.borrow_mut();
    *held += 1;
    println!("{}", *held);
    tries(task)
}

fn main() {
    let task = Task {
        job: Job::Queued { tries: RefCell::new(1) },
    };
    println!("{} {}", tries(&task), tries(&Task { job: Job::Done }));
    if let Job::Queued { tries: cell } = &task.job {
        println!("{}", bump(cell, &task));
    }
}
