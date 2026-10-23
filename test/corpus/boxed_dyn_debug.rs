// A `Box<dyn Debug>` is the string its value shows, as a `&dyn Debug` is
// (ADR 0060): made where it's boxed, shown by `{:?}`. So is an `Rc` that
// isn't counted; a counted one, a library's, is refused.
use std::fmt::Debug;

#[derive(Debug)]
struct P {
    x: i32,
}

fn shown(x: Box<dyn Debug>) -> String {
    format!("{x:?}")
}

fn main() {
    let a: Box<dyn Debug> = Box::new(0.3f32);
    let b: Box<dyn Debug> = Box::new(Some(6u8));
    let c: Box<dyn Debug> = Box::new("s");
    let d: Box<dyn Debug> = Box::new(None::<i32>);
    let e: Box<dyn Debug> = Box::new(P { x: 1 });
    println!("{a:?} {b:?} {c:?} {d:?} {e:?}");
    let all: Vec<Box<dyn Debug>> = vec![Box::new(1i8), Box::new((1, "a"))];
    println!("{all:?} {}", shown(Box::new([1, 2])));
}
