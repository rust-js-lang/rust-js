// A thread-local `Cell<f64>` counter is whole, whose JS text is Rust's
// (rd707, ADR 0994, amended): react-noop-renderer's flight-modules'
// `MODULE_IDX`, made `0.0` and set one more each time, `'' + moduleIdx++`
// upstream; one made by an `init` function, and one taken, which leaves
// `0.0`, too. One made `-0.0` or past 2^53, set past it by `replace`, given
// what isn't known whole, set through `with`, lent to a function, and a
// `pub` one keep Rust's text of any `f64`.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::thread::LocalKey;

thread_local! {
    static MODULE_IDX: Cell<f64> = const { Cell::new(0.0) };
    static MODULES: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
    static DOWN: Cell<f64> = Cell::new(1.0);
    static SIGNED: Cell<f64> = const { Cell::new(-0.0) };
    static FAR: Cell<f64> = Cell::new(1e21);
    static REPLACED: Cell<f64> = const { Cell::new(0.0) };
    static GIVEN: Cell<f64> = const { Cell::new(0.0) };
    static TAKEN: Cell<f64> = const { Cell::new(4.0) };
    static WITHIN: Cell<f64> = const { Cell::new(0.0) };
    static LENT: Cell<f64> = const { Cell::new(0.0) };
    pub static SHARED: Cell<f64> = const { Cell::new(0.0) };
}

fn save_module(render: u32) -> String {
    let idx = MODULE_IDX.get().to_string();
    MODULE_IDX.set(MODULE_IDX.get() + 1.0);
    MODULES.with_borrow_mut(|modules| modules.insert(idx.clone(), render));
    idx
}

fn read_module(idx: &str) -> Option<u32> {
    MODULES.with_borrow(|modules| modules.get(idx).copied())
}

fn step_down() -> String {
    DOWN.set(DOWN.get() - 1.0);
    format!("{}", DOWN.get())
}

fn set_far(local: &'static LocalKey<Cell<f64>>) {
    local.set(1e21);
}

fn given(n: f64) -> String {
    GIVEN.set(n);
    GIVEN.get().to_string()
}

pub fn main() {
    let a = save_module(3);
    let b = save_module(4);
    println!("{a} {b} {:?} {:?}", read_module("1"), read_module("2"));
    println!("{} {} {}", step_down(), step_down(), step_down());
    let signed = SIGNED.get();
    let far = FAR.get();
    println!("{signed} {far}");
    let before = REPLACED.replace(1e21);
    let after = REPLACED.get();
    println!("{before} {after}");
    println!("{} {}", given(2.0), given(1e300));
    let taken = TAKEN.take();
    let left = TAKEN.get();
    println!("{taken} {left}");
    WITHIN.with(|cell| cell.set(1e22));
    let within = WITHIN.get();
    println!("{within}");
    set_far(&LENT);
    let lent = LENT.get();
    println!("{lent}");
    SHARED.set(SHARED.get() + 1.0);
    let shared = SHARED.get();
    println!("{shared}");
}
