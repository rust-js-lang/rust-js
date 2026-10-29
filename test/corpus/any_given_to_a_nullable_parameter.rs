// A binding's `&dyn Any` is any JS value, given the value, not the pair
// that downcasts it (ADR 0331), of a `#[rust_js::nullable]` parameter too,
// webapi's `animate(keyframes: Option<&dyn Any>, ..)`: `Some(&frames)` is
// `frames`, a `None` `null` (rd170). Natively, the binding's body says what
// JS's `JSON.stringify` makes of each.
use std::any::Any;

struct Keyframes {
    opacity: [f64; 2],
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[cfg_attr(rust_js, rust_js::nullable(value))]
fn stringify(value: Option<&dyn Any>) -> String {
    match value {
        None => "null".to_string(),
        Some(v) => {
            if let Some(k) = v.downcast_ref::<Keyframes>() {
                format!("{{\"opacity\":[{},{}]}}", k.opacity[0], k.opacity[1])
            } else if let Some(n) = v.downcast_ref::<f64>() {
                format!("{n}")
            } else if let Some(s) = v.downcast_ref::<&str>() {
                format!("\"{s}\"")
            } else {
                "?".to_string()
            }
        }
    }
}

fn mapped(frames: Option<&Keyframes>) -> String {
    stringify(frames.map(|f| f as &dyn Any))
}

fn main() {
    println!("{}", stringify(Some(&Keyframes { opacity: [0.0, 1.0] })));
    println!("{}", stringify(Some(&2.5)));
    println!("{}", stringify(Some(&"text")));
    println!("{}", stringify(None));
    let frames = Keyframes { opacity: [1.0, 0.0] };
    println!("{}", mapped(Some(&frames)));
    println!("{}", mapped(None));
}
