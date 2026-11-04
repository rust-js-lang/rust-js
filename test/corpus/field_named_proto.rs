//@ compile-fail: a field whose JS name is `__proto__`, `__proto__` of `Linked`
// A field named `__proto__`, a Rust name as any other, can't be JS's: an
// object's `__proto__` is `Object.prototype`'s accessor, whose set, a
// constructor's `this.__proto__ = ..` or a copy's, makes the value the
// object's prototype, and whose read, of an object its `None` is left out
// of, gives the prototype (rd754). Refused, saying what to write.

#[derive(Debug, Default)]
struct Linked {
    __proto__: Option<u32>,
    size: u32,
}

fn main() {
    let linked = Linked::default();
    println!("{linked:?} {}", linked.__proto__.is_none());
}
