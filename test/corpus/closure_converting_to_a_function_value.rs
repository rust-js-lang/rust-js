//@ library-differs
// A closure that only converts what it's given, or what its call gives
// back, by conversions that are the value itself in JS, and passes it on to
// a function value it captured, is that function where it's made (ADR
// 0605, amended): alien-signals' `computed<T>` boxes its getter so, as Rust
// can name it no other way, its types not the getter's. One whose
// conversion isn't the value, `x + 1`, keeps its arrow, and so does one of
// `()` that discards what its call gives.
use std::rc::Rc;

struct Node<T> {
    compute: Box<dyn Fn(Option<Rc<T>>) -> Rc<T>>,
}

fn computed<T: 'static>(getter: impl Fn(Option<&T>) -> T + 'static) -> Node<T> {
    Node {
        compute: Box::new(move |previous: Option<Rc<T>>| Rc::new(getter(previous.as_deref()))),
    }
}

struct Stepper {
    step: Box<dyn Fn(u32) -> u32>,
    report: Box<dyn Fn(u32)>,
}

fn stepper(next: impl Fn(u32) -> u32 + 'static, log: impl Fn(u32) -> u32 + 'static) -> Stepper {
    Stepper {
        step: Box::new(move |n| next(n + 1)),
        report: Box::new(move |n| {
            log(n);
        }),
    }
}

fn main() {
    let count = computed(|previous: Option<&u32>| previous.map_or(1, |n| n * 2));
    let first = (count.compute)(None);
    let second = (count.compute)(Some(Rc::clone(&first)));
    println!("{first} {second}");
    let named = computed(|previous: Option<&String>| match previous {
        Some(name) => format!("{name}!"),
        None => "signal".to_string(),
    });
    let name = (named.compute)(None);
    println!("{} {}", name, (named.compute)(Some(Rc::clone(&name))));
    let s = stepper(
        |n| n * 10,
        |n| {
            println!("log {n}");
            n
        },
    );
    println!("step {}", (s.step)(2));
    (s.report)(3);
}
