// An `Option`'s `unwrap()` or `expect(..)` whose panic drops what's there
// as it unwinds, in a `try` no scope's `finally` takes: its test and panic,
// as an `assert!`'s are, the drops before the throw, and the option its
// value, called where it's written (ADR 0873, amended). React's
// `enqueueTask`, whose task is dropped where no impl is set, and handed
// over where one is. An option made by a call, or an `expect` whose
// message is read, stays its `try`, as does one a scope's `finally` drops
// around.
use std::cell::Cell;
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

type EnqueueTask = fn(Box<dyn Fn()>);

thread_local! {
    static ENQUEUE_TASK_IMPL: Cell<Option<EnqueueTask>> = const { Cell::new(None) };
    static MESSAGE: Cell<&'static str> = const { Cell::new("no impl") };
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

fn enqueue_task(task: Box<dyn Fn()>) {
    ENQUEUE_TASK_IMPL.get().unwrap()(task)
}

fn expected(task: Box<dyn Fn()>) {
    ENQUEUE_TASK_IMPL.get().expect("no impl given")(task)
}

/// Its message read before the drops, which change it.
struct Changing;

impl Drop for Changing {
    fn drop(&mut self) {
        MESSAGE.set("changed");
        println!("drop changing");
    }
}

fn read_first(task: Box<dyn Fn()>) {
    let changing = Changing;
    ENQUEUE_TASK_IMPL.get().expect(MESSAGE.get())(task);
    drop(changing);
}

/// The impl, counted: read once.
fn counted() -> Option<EnqueueTask> {
    CALLS.set(CALLS.get() + 1);
    ENQUEUE_TASK_IMPL.get()
}

fn once(task: Box<dyn Fn()>) {
    counted().unwrap()(task);
    println!("called {}", CALLS.get());
}

/// One a scope drops around, and a boxed `Some`, `Some(None)`'s, whose
/// value is its box's.
fn nested(value: Option<Option<u32>>) -> Option<u32> {
    let _held = Noisy("nested");
    value.unwrap()
}

fn task(name: &'static str) -> Box<dyn Fn()> {
    let noisy = Noisy(name);
    Box::new(move || println!("run {}", noisy.name()))
}

fn caught(name: &str, f: impl FnOnce()) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => println!("{name}: returned"),
        Err(payload) => {
            let message = (payload.downcast_ref::<String>().map(String::as_str))
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("?");
            println!("{name}: {message}");
        }
    }
}

fn main() {
    caught("unset", || enqueue_task(task("unset")));
    caught("unexpected", || expected(task("unexpected")));
    caught("read first", || read_first(task("read first")));
    caught("once", || once(task("once")));
    ENQUEUE_TASK_IMPL.set(Some(|task| task()));
    caught("set", || enqueue_task(task("set")));
    caught("expected", || expected(task("expected")));
    caught("read first, set", || read_first(task("read first, set")));
    caught("once, set", || once(task("once, set")));
    println!("{:?}", nested(Some(None)));
    caught("nested", || println!("{:?}", nested(None)));
}
