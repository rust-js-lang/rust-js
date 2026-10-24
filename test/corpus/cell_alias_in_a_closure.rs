// A `&Cell` a variable holds, of a field reached through a reference, set
// in a closure (#192): the field is what's set, each time the closure's
// called, not a copy of what it held.
use std::cell::Cell;

pub struct Context {
    pub value: Cell<u32>,
    pub label: Cell<&'static str>,
}

fn reset(context: &Context) {
    let r = &context.value;
    let again = || r.set(5);
    again();
}

fn bump_twice(context: &Context) -> u32 {
    let r = &context.value;
    let bump = || r.set(r.get() + 1);
    bump();
    bump();
    r.get()
}

fn relabel(context: &Context, labels: &[&'static str]) {
    let label = &context.label;
    labels.iter().for_each(|&l| label.set(l));
}

fn main() {
    let context = Context {
        value: Cell::new(1),
        label: Cell::new("first"),
    };
    reset(&context);
    println!("{}", context.value.get());
    println!("{} {}", bump_twice(&context), context.value.get());
    relabel(&context, &["a", "b", "last"]);
    println!("{}", context.label.get());
}
