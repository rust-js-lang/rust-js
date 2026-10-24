// An object that refers to itself, as React's context is its own
// `Provider` and its `Consumer`'s `_context`: an `Rc`, whose `Weak`s, set
// after it's made, break the cycle, as the Rust book teaches. Nothing asks
// whether one is alive, `upgrade` or a count, so each is the object
// itself, `context.provider = context;`, as upstream writes it, and a
// `Weak::new()` is `null` (ADR 0843). What it refers to is the same
// object, so a change through one is seen through the other.

use std::cell::Cell;
use std::rc::{Rc, Weak};

struct Consumer {
    kind: &'static str,
    context: Weak<Context>,
}

struct Context {
    value: Cell<f64>,
    provider: Cell<Weak<Context>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    consumer: Cell<Option<Rc<Consumer>>>,
}

fn create_context(value: f64) -> Rc<Context> {
    let context = Rc::new(Context {
        value: Cell::new(value),
        // These are circular
        provider: Cell::new(Weak::new()),
        consumer: Cell::new(None),
    });
    context.provider.set(Rc::downgrade(&context));
    context.consumer.set(Some(Rc::new(Consumer {
        kind: "consumer",
        context: Rc::downgrade(&context),
    })));
    context
}

fn main() {
    let context = create_context(1.5);
    context.value.set(2.5);
    println!("{}", context.value.get());
    let consumer = context.consumer.take().expect("a consumer");
    context.consumer.set(Some(consumer.clone()));
    println!("{}", consumer.kind);
    // The same allocation: a `Weak`'s identity, which needs no count.
    // `take` leaves a `Weak::new()`, `Default`'s, in its place.
    let provider = context.provider.take();
    println!("{}", Weak::ptr_eq(&context.provider.take(), &Weak::new()));
    context.provider.set(provider.clone());
    println!("{}", Weak::ptr_eq(&provider, &consumer.context));
    println!("{}", Weak::ptr_eq(&provider, &Weak::new()));
    let (none, other): (Weak<Context>, Weak<Context>) = (Weak::new(), Weak::new());
    println!("{}", Weak::ptr_eq(&none, &other));
    let lonely = Rc::new(Context {
        value: Cell::new(0.0),
        provider: Cell::new(Weak::new()),
        consumer: Cell::new(None),
    });
    println!("{}", Weak::ptr_eq(&Rc::downgrade(&lonely), &provider));
    println!("{:?}", provider);
}
