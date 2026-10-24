// A `()` given to JS as a value of any shape is `undefined`, what `()` is
// everywhere (ADR 0020): `x === undefined` of `strict_equals(x, ())`,
// where it was `x === []`, which no value is (ADR 0775). By value, by
// reference, as a `dyn` of any value, held in a variable, and given to
// a binding taken as a value. Each binding's native body answers as JS
// does, of `()` as of `undefined`.

use std::any::Any;

/// What JS says a value is, `typeof value`: of `()`, `"undefined"`.
fn kind(value: &dyn Any) -> &'static str {
    if value.is::<()>() {
        "undefined"
    } else if value.is::<f64>() || value.is::<u8>() {
        "number"
    } else if value.is::<&str>() {
        "string"
    } else {
        "object"
    }
}

/// `a === b`, of any two values: two `()`s are, as two `undefined`s are.
#[cfg_attr(rust_js, rust_js::link_name = "===")]
fn strict_equals<A: Any, B: Any>(this: A, other: B) -> bool {
    kind(&this) == "undefined" && kind(&other) == "undefined"
}

/// `typeof value`, of any value given by value.
#[cfg_attr(rust_js, rust_js::link_name = "typeof")]
fn type_of<T: Any>(this: T) -> String {
    kind(&this).to_string()
}

/// `typeof value`, of any value given as a `dyn`, as TypeScript's `any`
/// is by reference.
#[cfg_attr(rust_js, rust_js::link_name = "typeof")]
fn type_of_dyn(this: &dyn Any) -> String {
    kind(this).to_string()
}

/// `typeof value`, of a parameter whose type is `()` itself.
#[cfg_attr(rust_js, rust_js::link_name = "typeof")]
fn type_of_unit(this: ()) -> String {
    kind(&this).to_string()
}

fn main() {
    println!("{} {}", strict_equals((), ()), strict_equals((), 1.0));
    println!("{} {}", strict_equals(1.0, ()), strict_equals("a", ()));
    let nothing = ();
    println!("{}", strict_equals(nothing, ()));
    println!("{} {} {}", type_of(()), type_of(nothing), type_of(1.0));
    println!("{} {}", type_of_dyn(&()), type_of_dyn(&"a"));
    let held: &dyn Any = &();
    println!("{}", type_of_dyn(held));
    println!("{}", type_of_unit(()));
    let typed: fn(()) -> String = type_of_unit;
    println!("{}", typed(nothing));
}
