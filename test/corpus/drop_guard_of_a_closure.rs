// alien-signals restores what `effect`, `computed`, `run` and `trigger`
// changed in a `try { .. } finally { .. }`. Rust writes a guard that runs a
// closure as it's dropped, scopeguard's `defer!` written by hand: its
// `Drop` calls the closure, which is the `finally` (#501). Each way out of
// the guard's scope runs it once, after the scope's value is made, an early
// return, a panic, and a panic of the closure itself; two guards run theirs
// in the reverse of the order they're made, and a closure that owns a value
// runs before that value's dropped. A generic `Drop` that reads what it's
// given for its type parameter, a dictionary and a drop, stays a function;
// and a guard whose `Drop` lends its field `&mut` writes it through that.

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

thread_local! {
    static ACTIVE: Cell<u32> = const { Cell::new(0) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn log(line: String) {
    LOG.with_borrow_mut(|log| log.push(line));
}

struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

struct Named<F: FnMut()> {
    on_drop: F,
}

impl<F: FnMut()> Drop for Named<F> {
    fn drop(&mut self) {
        (self.on_drop)();
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        log(format!("drop {}", self.0));
    }
}

struct Holder<T: Default>(T);

impl<T: Default> Drop for Holder<T> {
    fn drop(&mut self) {
        let old = std::mem::take(&mut self.0);
        log("taken".to_string());
        drop(old);
    }
}

impl Default for Loud {
    fn default() -> Self {
        Loud("default")
    }
}

fn hold(name: &'static str) {
    let _held = Holder(Loud(name));
    log(format!("holding {name}"));
}

struct Back<'a>(&'a mut u32);

fn twice(n: &mut &mut u32) {
    **n *= 2;
}

impl Drop for Back<'_> {
    fn drop(&mut self) {
        twice(&mut self.0);
        log(format!("back {}", self.0));
    }
}

fn doubled(mut n: u32) -> u32 {
    {
        let _back = Back(&mut n);
    }
    n
}

pub struct Node {
    pub getter: Box<dyn Fn() -> u32>,
    pub depth: Cell<u32>,
}

fn oper(node: &Rc<Node>) -> u32 {
    node.depth.set(node.depth.get() + 1);
    let _restore = Finally(|| node.depth.set(node.depth.get() - 1));
    (node.getter)()
}

fn computed(getter: impl Fn() -> u32 + 'static) -> (Rc<Node>, impl Fn() -> u32) {
    let node = Rc::new(Node {
        getter: Box::new(getter),
        depth: Cell::new(0),
    });
    let held = Rc::clone(&node);
    (node, move || oper(&held))
}

fn run(active: u32, early: bool, fail: bool) -> u32 {
    let prev = ACTIVE.get();
    ACTIVE.set(active);
    let _restore = Finally(|| {
        log(format!("restore {prev} from {}", ACTIVE.get()));
        ACTIVE.set(prev);
    });
    if early {
        return ACTIVE.get() * 10;
    }
    if fail {
        panic!("failed in {}", ACTIVE.get());
    }
    ACTIVE.get() + 1
}

fn nested(n: u32) -> u32 {
    let mut seen = 0;
    let _outer = Named {
        on_drop: || log(format!("outer {n}")),
    };
    let _inner = Finally(|| {
        seen += 1;
        log(format!("inner {n} {seen}"));
    });
    for i in 0..n {
        if i == 2 {
            return i;
        }
    }
    n
}

fn owning(name: &'static str) {
    let loud = Loud(name);
    let _guard = Finally(move || log(format!("closure of {}", loud.0)));
    log(format!("body {name}"));
}

fn counted(n: u32) -> u32 {
    let start = n * 2;
    let _guard = Finally(|| log(format!("counted from {start}")));
    n + start
}

fn throwing_closure(fail: bool) -> u32 {
    let _guard = Finally(|| {
        if fail {
            panic!("the closure failed");
        }
        log("the closure ran".to_string());
    });
    log("before the drop".to_string());
    7
}

fn main() {
    let (node, get) = computed(|| 42);
    println!("{} {}", get(), node.depth.get());
    let (inner, read_depth) = computed(|| 0);
    let deep = Rc::clone(&inner);
    let (outer, get_outer) = computed(move || read_depth() + deep.depth.get() * 100);
    println!("{} {} {}", get_outer(), outer.depth.get(), inner.depth.get());
    let (failing, get_failing) = computed(|| panic!("getter failed"));
    let caught = catch_unwind(AssertUnwindSafe(&get_failing));
    println!("{} {}", caught.is_err(), failing.depth.get());

    println!("{} {}", run(3, false, false), ACTIVE.get());
    println!("{} {}", run(4, true, false), ACTIVE.get());
    let caught = catch_unwind(|| run(5, false, true));
    println!("{} {}", caught.is_err(), ACTIVE.get());

    println!("{}", nested(1));
    println!("{}", nested(5));
    owning("a");
    hold("b");
    println!("{}", counted(4));
    println!("{}", doubled(21));

    println!("{}", throwing_closure(false));
    let caught = catch_unwind(|| throwing_closure(true));
    println!("{}", caught.is_err());

    for line in LOG.take() {
        println!("{line}");
    }
}
