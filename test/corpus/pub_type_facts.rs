//@ library-differs
// `pub` generic functions that ask a fact of their type parameter, its
// size, name, `TypeId` and whether it needs a drop, and one of a bound's
// dictionary: their `.d.ts` leaves them out, as a JS caller can't give what
// each takes (ADR 1077), and a Rust caller, which gives it, calls them as
// before.
use std::any::{Any, TypeId};
use std::fmt::Display;

pub fn size<T>() -> usize {
    size_of::<T>()
}

pub fn named<T>() -> &'static str {
    std::any::type_name::<T>()
}

pub fn is_u32<T: 'static>(_value: T) -> bool {
    TypeId::of::<T>() == TypeId::of::<u32>()
}

pub fn boxed<T: 'static>(value: T) -> Box<dyn Any> {
    Box::new(value)
}

pub fn drops<T>() -> bool {
    std::mem::needs_drop::<T>()
}

pub fn shown<T: Display>(value: T) -> String {
    format!("<{value}>")
}

fn main() {
    println!("{} {} {}", size::<u8>(), size::<u32>(), size::<(u16, u16)>());
    println!("{} {}", named::<u32>(), named::<String>());
    println!("{} {}", is_u32(7u32), is_u32("seven"));
    let any = boxed(5u32);
    println!("{:?} {:?}", any.downcast_ref::<u32>(), any.downcast_ref::<i64>());
    println!("{} {}", drops::<u32>(), drops::<String>());
    println!("{} {}", shown(3), shown("three"));
}
