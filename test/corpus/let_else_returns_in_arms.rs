// A `let .. else { return; }` first in each arm of a `match`, whose arms
// return early from inside their `if`s too and go on to what's after the
// `match`; and in a `catch_unwind`'s closure, with more after it. MIR
// joins the returns into one, past a block of each arm and a block of the
// `if`s inside it: each is the `return` the program wrote, where it wrote
// it, `if (out.length > 3) { log(out, "many"); return; }`, never `bb51: {
// .. break bb51; .. }` (rd499). React's Fizz renderNode and
// flushCompletedQueues.
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn render(n: u32) -> u32 {
    if n < 2 {
        panic!("small {n}");
    }
    n
}

fn render_node(out: &mut Vec<String>, segment: Option<u32>, n: u32, aborted: bool) {
    let thrown;
    match segment {
        None => {
            let Err(thrown_value) = panic::catch_unwind(AssertUnwindSafe(|| render(n))) else {
                return;
            };
            out.push("replay".to_string());
            thrown = thrown_value;
            if aborted {
            } else if n == 0 {
                if out.len() > 3 {
                    out.push("many".to_string());
                    return;
                }
                if out.len() > 1 {
                    out.push("some".to_string());
                    return;
                }
            }
        }
        Some(segment) => {
            let Err(thrown_value) = panic::catch_unwind(AssertUnwindSafe(|| render(n + segment))) else {
                return;
            };
            out.push("render".to_string());
            thrown = thrown_value;
            if aborted {
                if out.len() > 3 {
                    out.push("many".to_string());
                    return;
                }
            } else if n == 0 {
                out.push("zero".to_string());
                return;
            }
        }
    }
    out.push("rethrow".to_string());
    panic::resume_unwind(thrown);
}

struct Request {
    pending: u32,
    root: RefCell<Option<Rc<Cell<u32>>>>,
    preamble: RefCell<Option<String>>,
    queue: RefCell<Vec<Rc<Cell<u32>>>>,
}

fn flush_item(out: &mut Vec<String>, item: &Cell<u32>) -> bool {
    out.push(item.get().to_string());
    item.get() != 0
}

fn flush_completed_queues(request: &Request, out: &mut Vec<String>) {
    out.push("begin".to_string());
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if request.pending > 0 {
            return;
        }
        let root = request.root.borrow().clone();
        if let Some(root) = root {
            if root.get() == 7 {
                return;
            }
            let Some(preamble) = request.preamble.borrow().clone() else {
                // The preamble isn't ready yet even though the root is.
                return;
            };
            out.push(preamble);
            if root.get() > 100 {
                panic!("big root {}", root.get());
            }
            request.root.replace(None);
        }
        let mut i = 0;
        while i < request.queue.borrow().len() {
            let item = Rc::clone(&request.queue.borrow()[i]);
            if !flush_item(out, &item) {
                i += 1;
                request.queue.borrow_mut().drain(0..i);
                return;
            }
            i += 1;
        }
        request.queue.borrow_mut().drain(0..i);
        out.push("end".to_string());
    }));
    out.push("finally".to_string());
    if let Err(error) = result {
        panic::resume_unwind(error);
    }
}

// A `return` written once after `if`s whose failing tests go on to it, past
// the drop of `s`: their ways stay its, `if (a != null) { if (a > 2 &&
// ..) { ..; return; } } // .. return;`, no copy of it in each; and one
// written after a guard of its own, in place.
fn written_once(a: Option<u32>, b: u32, out: &mut Vec<String>) {
    let s = format!("{b}");
    if b > 3 {
        if let Some(x) = a {
            if x > 2 && s.len() > 1 {
                out.push("two".into());
                return;
            }
        }
        // Written before the last return.
        return;
    }
    out.push(s);
}

fn written_after_a_guard(a: Option<u32>, b: u32, out: &mut Vec<String>) {
    let s = format!("{b}");
    if let Some(x) = a
        && x > b
    {
        out.push("gt".into());
        return;
    }
    if b > 3 {
        if s.len() > 1 && b != 50 {
            out.push("two".into());
            return;
        }
        // Written before the last return.
        out.push("one".into());
        return;
    }
    out.push(s);
}

// A `match` whose `_` arm returns, its comment above the `return`, before
// what drops: its guard first, as written, `if (target == null ||
// target.state !== state) { // ..\n return; }`, not the rest in `if (target
// != null && ..) { .. }` under the comment. React DOM's
// FormActionEventPlugin's extractEvents.
struct Fiber {
    state: u32,
}

fn extract_events(event: &str, target: Option<Rc<Fiber>>, state: u32, queue: &mut Vec<String>) {
    if event != "submit" {
        return;
    }
    let form = match target {
        Some(target) if target.state == state => target,
        _ => {
            // If we're inside a parent root that itself is a parent of this root, then
            // its deepest target won't be the actual form that's being submitted.
            return;
        }
    };
    let action = format!("action {}", form.state);
    queue.push(action);
}

fn main() {
    let cases: [(Option<u32>, u32, bool, usize); 10] = [
        (None, 5, false, 0),
        (None, 0, false, 0),
        (None, 0, false, 1),
        (None, 0, false, 3),
        (None, 0, true, 2),
        (Some(0), 1, false, 0),
        (Some(0), 0, false, 0),
        (Some(1), 0, true, 3),
        (Some(1), 0, true, 0),
        (Some(4), 0, true, 0),
    ];
    for (segment, n, aborted, filled) in cases {
        let mut out: Vec<String> = (0..filled).map(|i| i.to_string()).collect();
        let unwound = panic::catch_unwind(AssertUnwindSafe(|| render_node(&mut out, segment, n, aborted)));
        println!("{segment:?} {n} {aborted}: {out:?} {}", unwound.is_err());
    }

    for (a, b) in [
        (None, 2),
        (None, 5),
        (Some(1), 50),
        (Some(3), 50),
        (Some(3), 5),
        (Some(60), 50),
    ] {
        let mut out = Vec::new();
        written_once(a, b, &mut out);
        written_after_a_guard(a, b, &mut out);
        println!("{a:?} {b}: {out:?}");
    }

    let mut queue = Vec::new();
    for (event, target, state) in [
        ("click", Some(1), 1),
        ("submit", None, 1),
        ("submit", Some(2), 1),
        ("submit", Some(3), 3),
    ] {
        extract_events(event, target.map(|state| Rc::new(Fiber { state })), state, &mut queue);
    }
    println!("{queue:?}");

    let requests = [
        (1, Some(3), Some("pre"), vec![4, 5]),
        (0, Some(7), Some("pre"), vec![4, 5]),
        (0, Some(3), None, vec![4, 5]),
        (0, Some(3), Some("pre"), vec![4, 0, 5]),
        (0, None, None, vec![4, 5]),
        (0, Some(300), Some("pre"), vec![4]),
    ];
    for (pending, root, preamble, queue) in requests {
        let request = Request {
            pending,
            root: RefCell::new(root.map(|r| Rc::new(Cell::new(r)))),
            preamble: RefCell::new(preamble.map(str::to_string)),
            queue: RefCell::new(queue.into_iter().map(|q| Rc::new(Cell::new(q))).collect()),
        };
        let mut out = Vec::new();
        let unwound = panic::catch_unwind(AssertUnwindSafe(|| flush_completed_queues(&request, &mut out)));
        println!(
            "{out:?} {:?} {:?} {}",
            request.root.borrow().as_ref().map(|r| r.get()),
            request.queue.borrow().iter().map(|q| q.get()).collect::<Vec<_>>(),
            unwound.is_err()
        );
    }
}
