//@ compile-fail: `clone_from` of `std::option::Option<Counted>`, whose std `clone_from` is its value's
// `clone_from` of an `Option` of a type whose `clone_from` is the crate's
// own: refused, as std's `Option::clone_from` would call it, which rust-js
// doesn't write yet, not written as `*dest = source.clone()` (#922).

struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Self {
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from");
        self.0 = source.0;
    }
}

fn main() {
    let source = Some(Counted(1));
    let mut target = Some(Counted(2));
    target.clone_from(&source);
    println!("{}", target.map_or(0, |c| c.0));
}
