// An enum variant's `RefCell` field holds its value by its declaration, as
// a struct's does (ADR 0863, amended, #503): alien-signals' effect node,
// `Effect { cleanup }`, flattened into its parent and told by its keys
// (ADR 1071), and a tagged enum's, of named and of unnamed fields, each
// made in place, borrowed and replaced through a pattern's binding, which
// is the field's own place, its variant replaced whole; and one moved out,
// a cell of its own, its field its one cell where it's borrowed.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type Cleanup = Option<Rc<dyn Fn() -> u32>>;

pub enum NodeKind {
    Signal {
        current: Cell<u32>,
    },
    Effect {
        run: fn() -> Cleanup,
        cleanup: RefCell<Cleanup>,
    },
    Scope,
}

pub struct Node {
    pub flags: Cell<i32>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: NodeKind,
}

pub enum Job {
    Queued {
        log: RefCell<Vec<String>>,
        tries: RefCell<u32>,
    },
    Paired(RefCell<u32>, u32),
    Done,
}

pub struct Task {
    pub name: &'static str,
    pub job: Job,
}

fn cleanup_of() -> Cleanup {
    Some(Rc::new(|| 7))
}

pub fn effect(run: fn() -> Cleanup) -> Node {
    Node {
        flags: Cell::new(1),
        kind: NodeKind::Effect {
            run,
            cleanup: RefCell::new(None),
        },
    }
}

// What upstream's `run` does: the old cleanup taken and run, the new one kept.
pub fn run(node: &Node) -> u32 {
    let NodeKind::Effect { run, cleanup } = &node.kind else {
        return 0;
    };
    let old = cleanup.borrow_mut().take();
    let ran = old.map_or(0, |f| f());
    *cleanup.borrow_mut() = run();
    ran
}

pub fn has_cleanup(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Effect { cleanup, .. } => cleanup.borrow().is_some(),
        _ => false,
    }
}

pub fn stop(node: &mut Node) {
    node.kind = NodeKind::Scope;
}

pub fn queued() -> Task {
    Task {
        name: "build",
        job: Job::Queued {
            log: RefCell::new(Vec::new()),
            tries: RefCell::new(0),
        },
    }
}

pub fn attempt(task: &Task, line: &str) -> u32 {
    match &task.job {
        Job::Queued { log, tries } => {
            log.borrow_mut().push(format!("{}: {line}", task.name));
            *tries.borrow_mut() += 1;
            *tries.borrow()
        }
        Job::Paired(count, step) => {
            *count.borrow_mut() += step;
            *count.borrow()
        }
        Job::Done => 0,
    }
}

pub fn log_len(task: &Task) -> usize {
    if let Job::Queued { log, .. } = &task.job {
        log.borrow().len()
    } else {
        0
    }
}

pub enum Outcome {
    Finished {
        lines: RefCell<Vec<String>>,
        tries: RefCell<u32>,
    },
    Counted(RefCell<u32>),
    Dropped,
}

pub fn retry(outcome: &Outcome) -> u32 {
    match outcome {
        Outcome::Finished { tries, .. } | Outcome::Counted(tries) => {
            *tries.borrow_mut() += 1;
            *tries.borrow()
        }
        Outcome::Dropped => 0,
    }
}

// Moved out whole: a cell of its own of what it held.
pub fn finish(outcome: Outcome) -> Vec<String> {
    match outcome {
        Outcome::Finished { lines, tries } => {
            let mut lines = lines.into_inner();
            lines.push(format!("tries {}", tries.into_inner()));
            lines
        }
        Outcome::Counted(count) => vec![format!("count {}", count.into_inner())],
        Outcome::Dropped => Vec::new(),
    }
}

// The binding is of the task it was bound of, whatever its variable is
// set to after.
pub fn moved_on<'a>(first: &'a Task, second: &'a Task) -> u32 {
    let mut task = first;
    let Job::Queued { tries, .. } = &task.job else {
        return 0;
    };
    task = second;
    *tries.borrow_mut() += 10;
    *tries.borrow() + attempt(task, "moved")
}

pub fn restart(task: &mut Task) {
    task.job = Job::Paired(RefCell::new(10), 5);
}

fn main() {
    let mut node = effect(cleanup_of);
    println!("{} {}", has_cleanup(&node), node.flags.get());
    println!("ran {}", run(&node));
    println!("{}", has_cleanup(&node));
    println!("ran {}", run(&node));
    stop(&mut node);
    println!("{} {}", has_cleanup(&node), run(&node));
    let signal = Node {
        flags: Cell::new(2),
        kind: NodeKind::Signal { current: Cell::new(3) },
    };
    if let NodeKind::Signal { current } = &signal.kind {
        current.set(current.get() + 1);
        println!("signal {} {}", current.get(), has_cleanup(&signal));
    }
    let mut task = queued();
    println!("{} {}", attempt(&task, "a"), attempt(&task, "b"));
    println!("{}", log_len(&task));
    let other = queued();
    println!("{} {}", moved_on(&task, &other), moved_on(&other, &task));
    restart(&mut task);
    println!("{} {}", attempt(&task, "c"), log_len(&task));
    let done = Task {
        name: "done",
        job: Job::Done,
    };
    println!("{}", attempt(&done, "z"));
    let finished = Outcome::Finished {
        lines: RefCell::new(vec!["built".to_string()]),
        tries: RefCell::new(1),
    };
    println!("{}", retry(&finished));
    println!("{:?}", finish(finished));
    let counted = Outcome::Counted(RefCell::new(4));
    println!("{} {:?}", retry(&counted), finish(counted));
    println!("{} {:?}", retry(&Outcome::Dropped), finish(Outcome::Dropped));
}
