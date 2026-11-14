//! A library's generic writer function that holds a `Debug` builder in a
//! variable and steps it in a loop, which its consumer compiles an instance
//! of from its optimized MIR, where the loop's exit, the builder's
//! `finish`, may come before the steps its body makes (#1026).
use std::fmt;

pub fn rows<T: fmt::Debug>(name: &str, items: &[T], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut row = f.debug_tuple(name);
    for x in items {
        row.field(x);
    }
    row.finish()
}
