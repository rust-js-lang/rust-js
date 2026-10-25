// A module's exported cells, which only it sets, read by another as they
// are now: each its `export let`, the reader's import of it read live, ES's
// binding, and a read kept before the module sets it again what it was then
// (ADR 0270, amended). A `RefCell` of an array is the array, which the
// reader changes in place.

mod fiber {
    use std::cell::{Cell, RefCell};

    thread_local! {
        pub static IS_RENDERING: Cell<bool> = const { Cell::new(false) };
        pub static CURRENT: Cell<Option<u32>> = const { Cell::new(None) };
        pub static NAME: RefCell<String> = const { RefCell::new(String::new()) };
        pub static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    pub fn set_is_rendering(rendering: bool) {
        IS_RENDERING.set(rendering);
    }

    pub fn set_current(fiber: Option<u32>, name: &str) {
        CURRENT.set(fiber);
        NAME.set(name.to_string());
    }
}

fn told() -> String {
    let rendering = fiber::IS_RENDERING.get();
    let current = fiber::CURRENT.get();
    let name = fiber::NAME.with_borrow(|name| name.clone());
    fiber::LOG.with_borrow_mut(|log| log.push(current.unwrap_or(0)));
    // What was read before is what it was then, not what it's set to now.
    fiber::set_is_rendering(!rendering);
    format!("{rendering} {current:?} {name:?} {}", probe::rendering_now())
}

mod probe {
    pub fn rendering_now() -> bool {
        super::fiber::IS_RENDERING.get()
    }
}

fn main() {
    println!("{}", told());
    fiber::set_is_rendering(true);
    fiber::set_current(Some(3), "App");
    println!("{}", told());
    fiber::set_current(None, "");
    println!("{}", told());
    println!("{:?}", fiber::LOG.with_borrow(|log| log.clone()));
}
