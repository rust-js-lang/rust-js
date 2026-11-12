// An `Option<&dyn Any>` given to a `#[rust_js::nullable]` parameter is
// `null` where it's `None`, however it's made: `map` of a closure inlined
// or not, a variable, or a call's result. The pair's `?.value` made `None`
// `undefined` (#1581). Natively, the binding's body says what JS's
// `JSON.stringify` makes of each.
use std::any::Any;

struct Keyframes {
    opacity: f64,
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[cfg_attr(rust_js, rust_js::nullable(value))]
fn stringify(value: Option<&dyn Any>) -> String {
    match value {
        None => "null".to_string(),
        Some(v) => match v.downcast_ref::<Keyframes>() {
            Some(k) => format!("{{\"opacity\":{}}}", k.opacity),
            None => "?".to_string(),
        },
    }
}

fn noted() {}

fn plain(frames: Option<&Keyframes>) -> String {
    stringify(frames.map(|f| f as &dyn Any))
}

fn with_statement(frames: Option<&Keyframes>) -> String {
    stringify(frames.map(|f| {
        noted();
        f as &dyn Any
    }))
}

fn given(value: Option<&dyn Any>) -> String {
    stringify(value)
}

fn chosen(frames: &Keyframes, pick: bool) -> Option<&dyn Any> {
    if pick {
        Some(frames as &dyn Any)
    } else {
        None
    }
}

fn called(frames: &Keyframes, pick: bool) -> String {
    stringify(chosen(frames, pick))
}

fn main() {
    let k = Keyframes { opacity: 0.5 };
    println!("{} {}", plain(None), with_statement(None));
    println!("{} {}", plain(Some(&k)), with_statement(Some(&k)));
    println!("{} {}", given(None), given(Some(&k)));
    println!("{} {}", called(&k, false), called(&k, true));
}
