// A closure's `TypeId` is its own: two closures', even of one signature,
// differ, and one closure's is the same each time it's asked.
use std::any::{Any, TypeId};

fn id<T: Any>(_: &T) -> TypeId {
    TypeId::of::<T>()
}

fn main() {
    let a = |x: u32| x + 1;
    let b = |x: u32| x + 1;
    println!("{}", id(&a) == id(&b));
    println!("{}", id(&a) == id(&a));
    let boxed: Box<dyn Any> = Box::new(a);
    println!("{}", boxed.is::<u32>());
    println!("{}", (*boxed).type_id() == id(&a));
    assert_ne!(id(&a), id(&b));
}
