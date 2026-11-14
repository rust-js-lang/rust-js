// #1840, #1843 (P0, ADR 0030, amended): Rust has one `None`, and where it
// leaves for JS as `undefined` alone, a native optional argument's, a
// `None` held as `null` is given `?? undefined`. A nullable parameter, field,
// result and thread-local each hold `None` as `null` (ADR 0861); given on to
// `String(value)`, a binding's optional argument, Rust's `None` is
// "undefined" and JS's `null` "null". Through a function of the crate's, a
// trait object, a closure std calls, a fn pointer, a `dyn Fn` the binding
// is or a closure is, and a closure a generic function calls, each answers
// as native Rust does.
use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::link_name = "String")]
fn text(value: Option<f64>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(v) => format!("{v}"),
    }
}

// The same, given only as a `dyn Fn`.
#[cfg_attr(rust_js, rust_js::link_name = "String")]
fn text_of(value: Option<f64>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(v) => format!("{v}"),
    }
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn shown(value: Option<f64>) -> String {
    text(value)
}

fn given_on(value: Option<f64>) -> String {
    text(value)
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn passed_on(value: Option<f64>) -> String {
    given_on(value)
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn none_held() -> Option<f64> {
    None
}

struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    value: Option<f64>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<f64>> = const { Cell::new(None) };
}

trait Shows {
    fn show(&self, value: Option<f64>) -> String;
}

struct Plain;

impl Shows for Plain {
    fn show(&self, value: Option<f64>) -> String {
        text(value)
    }
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn through_dyn(shows: &dyn Shows, value: Option<f64>) -> String {
    shows.show(value)
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn through_pointer(f: fn(Option<f64>) -> String, value: Option<f64>) -> String {
    f(value)
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn through_callback(f: &dyn Fn(Option<f64>) -> String, value: Option<f64>) -> String {
    f(value)
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn through_binding(f: &dyn Fn(Option<f64>) -> String, value: Option<f64>) -> String {
    f(value)
}

fn apply<F: Fn(Option<f64>) -> String>(f: F, value: Option<f64>) -> String {
    f(value)
}

fn main() {
    println!("{}", shown(None));
    println!("{}", shown(Some(2.0)));
    println!("{}", passed_on(None));
    println!("{}", text(none_held()));
    let slot = Slot { value: None };
    println!("{}", text(slot.value));
    CURRENT.with(|current| println!("{}", text(current.get())));
    let mut values = vec![Some(1.5), None];
    values.push(none_held());
    let texts: Vec<String> = values.into_iter().map(text).collect();
    println!("{}", texts.join(","));
    println!("{}", through_dyn(&Plain, None));
    println!("{}", through_pointer(text, None));
    println!("{}", through_callback(&text, None));
    println!("{}", through_callback(&|value| text(value), None));
    println!("{}", through_binding(&text_of, None));
    println!("{}", apply(|value| text(value), none_held()));
    println!("{}", text(Some(3.0)));
}
