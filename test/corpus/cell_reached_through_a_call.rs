// A `Cell` a call reaches, `get(&c).value`, is reached once each time it's
// used (#191): `replace`, `take`, `update` and `swap` of it, and a helper
// given `&get(&c).value`, call `get` once, as Rust does, however often
// they read and write the cell; so does a `Cell` a call returns.
use std::cell::Cell;

pub struct Context {
    pub value: Cell<u32>,
    pub other: Cell<u32>,
}

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

fn get(c: &Context) -> &Context {
    CALLS.with(|n| n.set(n.get() + 1));
    c
}

fn calls() -> u32 {
    CALLS.with(|n| n.get())
}

fn value_of(c: &Context) -> &Cell<u32> {
    CALLS.with(|n| n.set(n.get() + 1));
    &c.value
}

fn bump(cell: &Cell<u32>) -> u32 {
    let v = cell.get() + 1;
    cell.set(v);
    cell.get()
}

fn main() {
    let c = Context {
        value: Cell::new(1),
        other: Cell::new(7),
    };
    let bumped = bump(&get(&c).value);
    println!("bump {bumped} {}", calls());
    get(&c).value.update(|v| v * 3);
    println!("update {} {}", c.value.get(), calls());
    let old = get(&c).value.replace(9);
    println!("replace {old} {}", calls());
    let taken = get(&c).value.take();
    println!("take {taken} {}", calls());
    get(&c).value.set(5);
    get(&c).value.swap(&get(&c).other);
    println!("swap {} {} {}", c.value.get(), c.other.get(), calls());
    let given = value_of(&c).replace(3);
    println!("returned {given} {} {}", c.value.get(), calls());
}
