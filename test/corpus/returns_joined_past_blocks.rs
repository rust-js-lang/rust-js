// Returns the program wrote in each branch, or in each of two inner loops,
// which MIR joins into one `return;` after a block of its own: each where
// it's written, no label, `} else if (tag === 2) { ..; return; }` and
// `while (grand != null) { if (..) { return; } .. }` (rd100, rd98: React
// DOM's attemptExplicitHydrationTarget and
// dispatchEventForPluginEventSystem). And guards one after another, each
// returning, MIR joins the same way: each `if (..) { ..; return; }` as
// written, not an `else` of the rest (the playground's new_file). And a
// `match` at a function's end, two arms of which return, the second's
// `return` as written though MIR puts the `_` arm before it (#435).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fiber {
    tag: u32,
    parent: Option<Rc<Fiber>>,
    containers: Vec<u32>,
}

fn at_priority(log: &RefCell<Vec<String>>, f: impl FnOnce()) {
    log.borrow_mut().push("priority".to_string());
    f();
}

fn instance_of(tag: u32) -> Option<u32> {
    if tag > 1 { Some(tag * 10) } else { None }
}

fn attempt(fiber: Option<Rc<Fiber>>, blocked: &Cell<Option<u32>>, log: &RefCell<Vec<String>>) {
    if let Some(fiber) = fiber {
        let nearest = fiber.parent.clone();
        if let Some(nearest) = nearest {
            let tag = nearest.tag;
            if tag == 1 || tag == 4 {
                let instance = instance_of(tag);
                if instance.is_some() {
                    blocked.set(instance);
                    at_priority(log, || {
                        log.borrow_mut().push(format!("hydrate {}", nearest.tag));
                    });
                    return;
                }
            } else if tag == 2 {
                let instance = instance_of(tag);
                if instance.is_some() {
                    blocked.set(instance);
                    at_priority(log, || {
                        log.borrow_mut().push(format!("hydrate {}", nearest.tag));
                    });
                    return;
                }
            } else if tag == 3 && instance_of(tag).is_some() {
                blocked.set(Some(0));
                return;
            }
        }
    }
    blocked.set(None);
}

fn walk(target: Option<Rc<Fiber>>, log: &RefCell<Vec<String>>) {
    let mut node = target;
    'main_loop: loop {
        let Some(current) = node else {
            return;
        };
        log.borrow_mut().push(format!("at {}", current.tag));
        let tag = current.tag;
        if tag == 3 || tag == 4 {
            if tag == 4 {
                let mut grand_node = current.parent.clone();
                while let Some(grand) = grand_node {
                    if grand.tag == 3 {
                        log.borrow_mut().push("grand root".to_string());
                        return;
                    }
                    grand_node = grand.parent.clone();
                }
            }
            for &container in &current.containers {
                if container == 0 {
                    return;
                }
                if container == 5 {
                    node = current.parent.clone();
                    log.borrow_mut().push("container".to_string());
                    continue 'main_loop;
                }
            }
        }
        node = current.parent.clone();
    }
}

fn new_file(answer: Option<&str>, existing: &[&str], log: &RefCell<Vec<String>>) {
    let answer = match answer {
        Some(answer) => answer,
        None => return,
    };
    let path = answer.trim().to_string();
    if path.is_empty() {
        return;
    }
    if !path.ends_with(".rs") {
        let text = format!("{path:?} isn't a Rust module file name.");
        log.borrow_mut().push(text);
        return;
    }
    if existing.contains(&path.as_str()) {
        log.borrow_mut().push(format!("{path} already exists."));
        return;
    }
    let module = path.strip_suffix(".rs").unwrap_or(&path);
    log.borrow_mut().push(format!("Created {path}, {module}."));
}

fn pop_host(fiber: &Fiber, log: &RefCell<Vec<String>>) {
    if let Some(parent) = fiber.parent.clone() {
        match parent.tag {
            5 | 13 => {
                log.borrow_mut().push("host".to_string());
                return;
            }
            3 | 27 => {
                log.borrow_mut().push(format!("root {}", parent.tag));
                return;
            }
            _ => {
                log.borrow_mut().push("other".to_string());
            }
        }
    }
}

fn fiber(tag: u32, parent: Option<Rc<Fiber>>, containers: Vec<u32>) -> Option<Rc<Fiber>> {
    Some(Rc::new(Fiber { tag, parent, containers }))
}

fn main() {
    let log = RefCell::new(Vec::new());
    let blocked = Cell::new(Some(99));
    for tag in [1, 2, 3, 4, 7] {
        attempt(fiber(0, fiber(tag, None, vec![]), vec![]), &blocked, &log);
        println!("{tag}: {:?} {:?}", blocked.get(), log.take());
    }
    attempt(None, &blocked, &log);
    println!("none: {:?}", blocked.get());
    let root = fiber(3, None, vec![1]);
    for start in [
        fiber(4, root.clone(), vec![]),
        fiber(4, fiber(9, None, vec![]), vec![2, 5]),
        fiber(3, fiber(4, root.clone(), vec![0]), vec![1, 1]),
        fiber(8, fiber(3, None, vec![5]), vec![]),
        None,
    ] {
        walk(start, &log);
        println!("{:?}", log.take());
    }
    for answer in [None, Some("  "), Some("a.txt"), Some(" b.rs "), Some("c.rs")] {
        new_file(answer, &["b.rs"], &log);
    }
    println!("{:?}", log.take());
    for tag in [5, 13, 3, 27, 0] {
        pop_host(&Fiber { tag: 0, parent: fiber(tag, None, vec![]), containers: vec![] }, &log);
    }
    pop_host(&Fiber { tag: 0, parent: None, containers: vec![] }, &log);
    println!("{:?}", log.take());
}
