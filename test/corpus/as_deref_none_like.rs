//@ compile-fail: for an `Option` of what could look like `None`
// A `Deref` to an `Option`: `Some(&None)`, which JS would have to box.
use std::ops::Deref;

struct Maybe(Option<i32>);

impl Deref for Maybe {
    type Target = Option<i32>;
    fn deref(&self) -> &Option<i32> {
        &self.0
    }
}

fn main() {
    let m = Some(Maybe(None));
    println!("{:?}", m.as_deref());
}
