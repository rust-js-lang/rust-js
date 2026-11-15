// A closure's capture its maker gives `null` or a value, a nullable field's
// `replace` kept, is that in the closure too, stored back as it is, React's
// Fizz retryRenderTask's `request.current_task.replace(prev_task.clone())`
// in its `catch_unwind`; one a closure writes, `prev = None`, keeps `??
// null`, so the crate's `=== null` reads Rust's `None` (#1933 step 4 (F)).
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

struct Task {
    id: u32,
}

struct Request {
    #[cfg_attr(rust_js, rust_js::nullable)]
    current_task: RefCell<Option<Rc<Task>>>,
}

fn retry(request: &Request, task: &Rc<Task>, fail: bool) -> u32 {
    let prev_task = request.current_task.replace(Some(Rc::clone(task)));
    let result = catch_unwind(AssertUnwindSafe(|| {
        if fail {
            request.current_task.replace(prev_task.clone());
            return 0;
        }
        task.id
    }));
    if !fail {
        request.current_task.replace(prev_task);
    }
    result.unwrap_or(1)
}

fn restored(request: &Request, task: &Rc<Task>, clear: bool) {
    let mut prev_task = request.current_task.replace(Some(Rc::clone(task)));
    let mut restore = move || {
        if clear {
            prev_task = None;
        }
        request.current_task.replace(prev_task.clone());
    };
    restore();
}

fn shown(request: &Request) -> String {
    match &*request.current_task.borrow() {
        Some(task) => format!("task {}", task.id),
        None => "none".to_string(),
    }
}

fn main() {
    let request = Request {
        current_task: RefCell::new(None),
    };
    let first = Rc::new(Task { id: 1 });
    let second = Rc::new(Task { id: 2 });
    println!("{} {}", retry(&request, &first, false), shown(&request));
    request.current_task.replace(Some(Rc::clone(&second)));
    println!("{} {}", retry(&request, &first, true), shown(&request));
    restored(&request, &first, false);
    println!("{} {}", shown(&request), request.current_task.borrow().is_some());
    restored(&request, &first, true);
    println!("{} {}", shown(&request), request.current_task.borrow().is_some());
}
