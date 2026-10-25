// A function handed to JS is JS's: a binding keeps the `Box<dyn Fn>` it's
// given, and JS has no drop, so nothing drops it after, and the closure
// carries no drop for it (ADR 0873). Where Rust still holds it, its drop
// runs: unwinding before the handoff, a later argument's panic among it,
// or a path that never hands it over. One a binding gives back, `kept`'s
// or `put`'s, is Rust's again, and drops. React's `enqueueTask`, whose
// `catch_unwind`, which never unwinds itself, is a plain `try`, and its
// `act`'s `queueSeveralMicrotasks`.
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Noisy {
    fn name(&self) -> &'static str {
        self.0
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// `task.call()`: JS calls the task and keeps it, as `setImmediate` and a
/// port's `onmessage` do; nothing drops it.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn run_kept(this: Box<dyn Fn()>) {
    this();
    std::mem::forget(this);
}

/// `callback.call()`: JS calls the callback once, as `queueMicrotask`
/// does, which uses up what it holds.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn run_once(this: Box<dyn FnOnce()>) {
    this();
}

/// `task.call(label)`: JS calls the task, its `this` the label, and keeps
/// it.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn run_kept_as(this: Box<dyn Fn()>, label: &str) {
    let _ = label;
    this();
    std::mem::forget(this);
}

/// The label, which mustn't be empty.
fn checked(label: &str) -> &str {
    assert!(!label.is_empty(), "no label");
    label
}

/// `Reflect.set(slot, "value", task)`: JS writes the task where it's told,
/// which is Rust's again.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.set")]
fn put(slot: &mut Option<Box<dyn Fn()>>, key: &str, task: Box<dyn Fn()>) -> bool {
    let _ = key;
    *slot = Some(task);
    true
}

/// The function itself, given back: Rust's again.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn kept(this: Box<dyn Fn()>) -> Box<dyn Fn()> {
    this
}

type EnqueueTask = fn(Box<dyn Fn()>);

thread_local! {
    static POSTED: Cell<u32> = const { Cell::new(0) };
    static PORTS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static ENQUEUE_TASK_IMPL: Cell<Option<EnqueueTask>> = const { Cell::new(None) };
}

fn enqueue_task(task: Box<dyn Fn()>, host: &str) {
    if ENQUEUE_TASK_IMPL.get().is_none()
        && let Err(_err) = panic::catch_unwind(|| {
            if host == "node" {
                ENQUEUE_TASK_IMPL.set(Some(run_kept));
            } else if host != "none" {
                panic!("no timers in {host}");
            }
        })
    {
        ENQUEUE_TASK_IMPL.set(Some(|callback| {
            let posted = POSTED.get() + 1;
            POSTED.set(posted);
            if posted > 2 {
                panic!("no port for post {posted}");
            }
            run_kept(Box::new(move || callback()));
        }));
    }
    assert!(!host.is_empty(), "no host");
    if let Some(enqueue_task_impl) = ENQUEUE_TASK_IMPL.get() {
        enqueue_task_impl(task);
    }
}

/// The task, handed to JS on the port at `at` if it's open: Rust's until
/// then, and dropped if the port is out of bounds or closed.
fn enqueue_on(task: Box<dyn Fn()>, at: usize) {
    let port = PORTS.with_borrow(|ports| ports[at]);
    if port > 0 {
        run_kept(task);
    }
}

fn queue_several_microtasks(callback: Box<dyn FnOnce()>) {
    run_once(Box::new(move || {
        run_once(callback);
    }));
}

fn task(name: &'static str) -> Box<dyn Fn()> {
    let noisy = Noisy(name);
    Box::new(move || println!("run {}", noisy.name()))
}

fn main() {
    for host in ["node", "browser", "none"] {
        ENQUEUE_TASK_IMPL.set(None);
        enqueue_task(task(host), host);
        enqueue_task(task(host), host);
        println!("enqueued in {host}");
    }
    ENQUEUE_TASK_IMPL.set(None);
    let unwound = panic::catch_unwind(AssertUnwindSafe(|| enqueue_task(task("posted"), "browser")));
    println!("posted: {}", unwound.is_err());
    let unwound = panic::catch_unwind(AssertUnwindSafe(|| enqueue_task(task("hostless"), "")));
    println!("hostless: {}", unwound.is_err());
    PORTS.with_borrow_mut(|ports| ports.extend([1, 0]));
    for at in [0, 1, 2] {
        let unwound = panic::catch_unwind(AssertUnwindSafe(|| enqueue_on(task("on a port"), at)));
        println!("port {at}: {}", unwound.is_err());
    }
    let noisy = Noisy("microtask");
    queue_several_microtasks(Box::new(move || println!("ran {}", noisy.name())));
    // Made before the label is checked: Rust's, and dropped, as that unwinds.
    for label in ["checked", ""] {
        let noisy = Noisy("labelled");
        let unwound = panic::catch_unwind(AssertUnwindSafe(|| {
            run_kept_as(Box::new(move || println!("run {}", noisy.name())), checked(label))
        }));
        println!("labelled {label:?}: {}", unwound.is_err());
    }
    let noisy = Noisy("given back");
    let given_back = kept(Box::new(move || println!("run {}", noisy.name())));
    given_back();
    drop(given_back);
    let mut slot: Option<Box<dyn Fn()>> = None;
    let noisy = Noisy("put back");
    put(&mut slot, "value", Box::new(move || println!("run {}", noisy.name())));
    if let Some(put_back) = &slot {
        put_back();
    }
    drop(slot);
    println!("end");
}
