// A value filled where it's absent, React's Fizz instruction set's
// view-transition name: `let name = match attribute(key).filter(..) {
// Some(name) => name, None => { ..; format!(..) } };`, one variable,
// `let name = attribute(key); if (!name) { name = .. }` (ADR 0849). Each
// given one kept, each missing or empty one made the next.
use std::cell::Cell;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(0) };
}

fn attribute(key: &str) -> Option<String> {
    match key {
        "set" => Some("given".to_string()),
        "empty" => Some(String::new()),
        _ => None,
    }
}

fn named(key: &str) -> String {
    let name = match attribute(key).filter(|name| !name.is_empty()) {
        Some(name) => name,
        None => {
            let index = NEXT.get();
            NEXT.set(index + 1);
            format!("_T_{index}_")
        }
    };
    name
}

pub fn main() {
    println!("{} {} {} {}", named("set"), named("empty"), named("none"), named("set"));
}
