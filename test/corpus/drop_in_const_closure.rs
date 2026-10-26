// A guard in a closure coerced to a `fn` in a `const`, as React's
// `callComponentInDEV` is: its `Drop` is its body where it's dropped, a
// `finally` that restores what the closure changed, however it's left
// (ADR 0608, amended).
use std::cell::{Cell, RefCell};

thread_local! {
    static IS_RENDERING: Cell<bool> = const { Cell::new(false) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn set_is_rendering(rendering: bool) {
    LOG.with_borrow_mut(|log| log.push(format!("set {rendering}")));
    IS_RENDERING.set(rendering);
}

struct Restore(bool);

impl Drop for Restore {
    fn drop(&mut self) {
        set_is_rendering(self.0);
    }
}

struct Labeled {
    label: String,
    depth: u32,
}

impl Drop for Labeled {
    fn drop(&mut self) {
        LOG.with_borrow_mut(|log| log.push(format!("leave {} at {}", self.label, self.depth)));
    }
}

const CALL: fn(&dyn Fn() -> u32) -> u32 = |f| {
    let was_rendering = IS_RENDERING.get();
    set_is_rendering(true);
    let _restore = Restore(was_rendering);
    f()
};

struct Depth<'a>(&'a Cell<u32>);

impl Drop for Depth<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

struct Calls;

impl Calls {
    /// An associated `const`'s closure, a guard of what it's given.
    const NESTED: fn(&Cell<u32>, &dyn Fn() -> u32) -> u32 = |depth, f| {
        depth.set(depth.get() + 1);
        let _depth = Depth(depth);
        f()
    };
}

const LABELED: fn(&str, u32) -> u32 = |label, depth| {
    let _labeled = Labeled {
        label: label.to_string(),
        depth,
    };
    if depth > 2 {
        return depth * 10;
    }
    CALL(&|| depth + u32::from(IS_RENDERING.get()))
};

fn main() {
    println!("{}", CALL(&|| u32::from(IS_RENDERING.get())));
    println!("{}", CALL(&|| CALL(&|| 7) + 1));
    for depth in 0..5 {
        println!("{}", LABELED("row", depth));
    }
    println!("{}", IS_RENDERING.get());
    let depth = Cell::new(0);
    let inner = (Calls::NESTED)(&depth, &|| (Calls::NESTED)(&depth, &|| depth.get() * 10) + depth.get());
    println!("{inner} {}", depth.get());
    for line in LOG.take() {
        println!("{line}");
    }
}
