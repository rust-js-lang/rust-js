//@ compile-fail: a value of a trait method's `impl Trait` beside another in its signature (#1320), where a type may have a destructor
// Two `impl Trait`s of one trait method's signature, which one word would
// name both of in its dictionary, so it carries the drop of neither:
// generic code holding one, where a type may have a destructor, is refused,
// naming the issue that would carry them (#1320).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Pair {
    fn both() -> (impl Sized, impl Sized);
}

struct Louds;

impl Pair for Louds {
    fn both() -> (impl Sized, impl Sized) {
        (Loud("a"), Loud("b"))
    }
}

fn held<T: Pair>() {
    let _kept = T::both();
    println!("holding");
}

fn main() {
    held::<Louds>();
}
