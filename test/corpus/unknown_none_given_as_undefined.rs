// A `None` of a value of any shape, `Option<&Unknown>`, is JS's `undefined`
// itself (rd275), given to a binding's optional argument as that value:
// `String(None)` is `String(undefined)`, "undefined", as `js::string`
// says, a literal one as a dynamic one, not left out, `String()`'s "".
// Natively, the binding's body says what JS's `String` makes of it.
use std::marker::PhantomData;

/// A JS value of any shape, as `js::Unknown` is.
#[cfg_attr(rust_js, rust_js::types = "unknown")]
struct Value(PhantomData<*mut ()>, u8);

#[cfg_attr(rust_js, rust_js::link_name = "String")]
fn text(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(_) => "a value".to_string(),
    }
}

fn given(value: Option<&Value>) -> String {
    text(value)
}

fn main() {
    println!("{}", text(None));
    println!("{}", given(None));
}
