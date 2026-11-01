use std::cell::Cell;

use dep::{Bag, Hooks};

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

fn use_id() -> String {
    let id = COUNT.get();
    COUNT.set(id + 1);
    format!("_S_{id}_")
}

pub static HOOKS_DISPATCHER: Hooks = Hooks {
    use_id,
    use_memo: |create| create() + 1,
    name: "server",
};

const EMPTY: Bag = Bag { items: Vec::new() };

pub fn main() {
    println!("{}", dep::render(&HOOKS_DISPATCHER));
    println!("{}", dep::render(&HOOKS_DISPATCHER));
    let mut first = EMPTY;
    first.items.push(1);
    let second = EMPTY;
    println!("{:?} {:?}", first.items, second.items);
}
