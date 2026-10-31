// A trailing `None` given a function value is no argument, `f()` of
// `f(None)`, as a person leaves an optional one out (ADR 0443, amended):
// a closure's, a `dyn Fn`'s, a pointer's, a generic one's, alone or after
// another argument. Only `arguments.length` tells, which no Rust function
// reads. A `None` before a given argument stays, and a `Some(())` is given.
// A pointer's trailing `()` is no argument either, as a closure's (ADR 0441);
// a unit struct's, `undefined` too, is a value given.
fn shown(x: Option<u32>) -> String {
    match x {
        Some(n) => format!("some {n}"),
        None => "none".to_string(),
    }
}

fn apply(f: &dyn Fn(Option<u32>) -> String, g: &dyn Fn(u32, Option<&str>)) -> String {
    g(1, None);
    g(2, Some("two"));
    f(None)
}

fn done(_: ()) {
    println!("done");
}

fn pointed(f: fn(Option<u32>) -> String, unit: fn(Option<()>) -> bool, finish: fn(())) -> String {
    finish(());
    format!("{} {} {} {}", f(None), f(Some(3)), unit(None), unit(Some(())))
}

struct Marker;

fn marked(f: &dyn Fn(u32, Marker)) {
    f(5, Marker);
}

fn each<F: Fn(Option<u32>, u32) -> String>(f: F) -> String {
    f(None, 4)
}

fn main() {
    let named = |x: Option<u32>| x.map_or(0, |n| n + 1);
    println!("{} {}", named(None), named(Some(1)));
    let g = |n: u32, label: Option<&str>| println!("g {n} {}", label.unwrap_or("none"));
    println!("{}", apply(&shown, &g));
    println!("{}", pointed(shown, |u| u.is_some(), done));
    marked(&|n, _| println!("marked {n}"));
    println!("{}", each(|x, n| format!("{} {n}", shown(x))));
}
